use scrypto::prelude::*;

blueprint! {
    struct BankTester {
        vault: Vault
    }

    impl BankTester {

        pub fn new(starting_cash: Bucket) -> Component {
            Self {
                vault: Vault::with_bucket(starting_cash),
            }
            .instantiate()
        }

        pub fn execute(&mut self, money: Bucket) -> Bucket {
            let mut returned_bucket = self.vault.take(money.amount()/100);
            returned_bucket.put(money);
            returned_bucket
        }

    }
}
