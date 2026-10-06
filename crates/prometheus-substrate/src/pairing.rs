//! Pairing tickets, and how a human moves one between two machines.
//!
//! # What pairing actually is here
//!
//! Admission is **not** discovery. mDNS and the Mainline DHT answer "where is
//! this EndpointId"; neither says anything about whether that peer may sync.
//! That decision is a pairing ticket: a base64url payload carrying the group
//! secret, the endpoint id, and the signing-key fingerprint, exported by
//! `P2PIdentity::export_ticket` and admitted by `import_ticket` plus
//! `authorize_endpoint`.
//!
//! This module adds only the transport for the human step. A ticket is ~200
//! characters of base64url; retyping it between two machines is exactly the
//! kind of task people get wrong silently, so it is offered as a scannable
//! `prometheus://pair?ticket=…` URI.
//!
//! # Why this produces a URI, not a QR image
//!
//! Rendering the QR bitmap needs an encoder crate, and every new Companion
//! dependency must first be pinned in `versions.toml` — a hand edit only the
//! operator can make (`.claude/settings.json` denies agents `Edit`). Rather
//! than block on that, this module produces the exact payload a QR encodes and
//! leaves the drawing to the UI layer, where a JS/WASM encoder is already the
//! natural choice and needs no Rust pin at all. The security-relevant part —
//! what goes *into* the code, and that it survives transport unaltered — is
//! here and tested.
//!
//! # No Noise handshake
//!
//! The architecture plan described pairing as "Noise + Iroh QUIC". That was
//! never true of this implementation: `grep -i noise` over the sovereign-sync
//! sources and manifest returns nothing. Peers authenticate by **Ed25519
//! EndpointId over iroh's QUIC/TLS 1.3**, and admission is the ticket plus the
//! endpoint allowlist. `change-cpc-006` corrects the spec rather than adding a
//! second handshake to match a document.
//!
//! # Not yet decided
//!
//! Whether v1 also offers a 6-digit short code (a PAKE such as `spake2`, the
//! way `clawdesk-discovery` does it) is an open question carried from analyze.
//! QR carries the whole ticket and needs no extra key exchange, so nothing here
//! blocks on that decision.

/// A pairing ticket rendered for display.
pub struct RenderedTicket {
    ticket: String,
}

impl RenderedTicket {
    /// Wrap an encoded ticket from `P2PIdentity::export_ticket`.
    ///
    /// Rejects an empty ticket: rendering a QR code of nothing produces a
    /// scannable image that pairs no one, which is worse than an error.
    pub fn new(ticket: impl Into<String>) -> anyhow::Result<Self> {
        let ticket = ticket.into();
        let trimmed = ticket.trim();
        if trimmed.is_empty() {
            anyhow::bail!("refusing to render an empty pairing ticket");
        }
        Ok(Self {
            ticket: trimmed.to_string(),
        })
    }

    /// The ticket text, for copy-paste and for the QR payload.
    pub fn text(&self) -> &str {
        &self.ticket
    }

    /// A `prometheus://pair?ticket=…` URI.
    ///
    /// The ticket is already base64url (`URL_SAFE_NO_PAD`), so it is safe in a
    /// query string without further escaping — asserted by a test rather than
    /// assumed, because a ticket that silently changed encoding would fail to
    /// import with a confusing error.
    pub fn uri(&self) -> String {
        format!("prometheus://pair?ticket={}", self.ticket)
    }

    /// The exact payload a QR code should encode.
    ///
    /// Identical to [`uri`](Self::uri); named separately so a UI reads as
    /// "encode this" rather than guessing which string to use.
    pub fn qr_payload(&self) -> String {
        self.uri()
    }

    /// Whether the payload fits a QR code a phone camera can read comfortably.
    ///
    /// Version 40 at error-correction level L tops out near 2,953 bytes, but
    /// codes that large need a good camera and steady hands. 1,200 bytes keeps
    /// a ticket inside roughly version 25, which scans reliably off a laptop
    /// screen. A ticket is normally ~250 bytes, so this is a guard against a
    /// future payload change, not a live constraint.
    pub fn fits_scannable_qr(&self) -> bool {
        self.qr_payload().len() <= 1_200
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A realistic ticket: base64url of a JSON pairing payload.
    const TICKET: &str = "eyJwcm90b2NvbF92ZXJzaW9uIjoiMSIsImdyb3VwX3NlY3JldCI6IkFBQUEifQ";

    #[test]
    fn an_empty_ticket_is_refused_rather_than_rendered() {
        assert!(RenderedTicket::new("   ").is_err());
        assert!(RenderedTicket::new("").is_err());
    }

    /// The ticket must survive the URI round trip byte-for-byte. If it did not,
    /// the receiving side's `import_ticket` would fail base64 decoding.
    #[test]
    fn the_uri_carries_the_ticket_unmodified() {
        let rendered = RenderedTicket::new(TICKET).expect("ticket");
        let uri = rendered.uri();
        let carried = uri
            .split_once("ticket=")
            .map(|(_, value)| value)
            .expect("uri must carry a ticket parameter");
        assert_eq!(
            carried, TICKET,
            "the ticket was altered by URI construction; import would fail"
        );
    }

    /// The payload is what a scanner will read back, so it must be exactly the
    /// URI — not a truncation, not a re-encoding.
    #[test]
    fn the_qr_payload_is_the_pairing_uri_and_fits_a_scannable_code() {
        let rendered = RenderedTicket::new(TICKET).expect("ticket");
        assert_eq!(rendered.qr_payload(), rendered.uri());
        assert!(rendered
            .qr_payload()
            .starts_with("prometheus://pair?ticket="));
        assert!(
            rendered.fits_scannable_qr(),
            "a realistic ticket must fit a comfortably scannable QR code"
        );
    }

    /// The size guard must actually reject something, or it is decoration.
    #[test]
    fn an_oversized_ticket_is_reported_as_unscannable() {
        let huge = RenderedTicket::new("A".repeat(2_000)).expect("ticket");
        assert!(
            !huge.fits_scannable_qr(),
            "a 2000-byte ticket should be flagged as too large to scan reliably"
        );
    }
}
