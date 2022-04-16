use radix_engine::ledger::*;
use radix_engine::transaction::*;
use scrypto::prelude::*;

#[test]
fn test_bank() {
    // Set up environment.
    let mut ledger = InMemorySubstateStore::with_bootstrap();
    let mut executor = TransactionExecutor::new(&mut ledger, false);
    let key = executor.new_public_key();
    let account = executor.new_account(key);
    let package = executor
        .publish_package(include_code!("growth_account"))
        .unwrap();

    // Test the Bank instantiation
    let transaction1 = TransactionBuilder::new(&executor)
        .call_function(package, "Bank", "new", vec!["0.09".to_owned()], None)
        .call_method_with_all_resources(account, "deposit_batch")
        .build(vec![key])
        .unwrap();
    let receipt1 = executor.run(transaction1).unwrap();
    println!("{:?}\n", receipt1);
    assert!(receipt1.result.is_ok());

    // Test the `deposit` method.
    let component = receipt1.component(0).unwrap();
    let transaction2 = TransactionBuilder::new(&executor)
        .call_method(
            component,
            "deposit",
            vec!["100,030000000000000000000000000000000000000000000000000004".to_owned()],
            Some(account),
        )
        .call_method_with_all_resources(account, "deposit_batch")
        .build(vec![key])
        .unwrap();
    let receipt2 = executor.run(transaction2).unwrap();
    println!("{:?}\n", receipt2);
    assert!(receipt2.result.is_ok());

    // Test the `withdraw` method.
    let component = receipt1.component(0).unwrap();
    let transaction2 = TransactionBuilder::new(&executor)
        .call_method(
            component,
            "withdraw",
            vec!["100,033ce7fff53e6789f4e9fc62b4af96dd9bebfb9212593b9482d7b0".to_owned()],
            Some(account),
        )
        .call_method_with_all_resources(account, "deposit_batch")
        .build(vec![key])
        .unwrap();
    let receipt2 = executor.run(transaction2).unwrap();
    println!("{:?}\n", receipt2);
    assert!(receipt2.result.is_ok());
}
