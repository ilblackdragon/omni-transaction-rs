pub enum TransactionType {
    /// Pay to public key hash
    P2PKH,
    /// Pay to script hash
    P2SH,
    /// Pay to witness public key hash
    P2WPKH,
    /// Pay to witness script hash
    P2WSH,
}
