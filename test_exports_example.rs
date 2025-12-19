#[cfg(feature = "near")]
fn main() {
    // Test that the re-exports work
    use omni_transaction::near::{AccountId, NearGas, NearToken};
    
    // These should compile
    let _token = NearToken::from_near(1);
    let _gas = NearGas::from_tgas(100);
    let _account: AccountId = "alice.near".parse().unwrap();
    
    println!("All exports work correctly!");
}

#[cfg(not(feature = "near"))]
fn main() {
    println!("NEAR feature not enabled");
}
