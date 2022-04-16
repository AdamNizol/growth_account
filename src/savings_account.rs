use scrypto::prelude::*;

blueprint! {
    struct SavingsAccount {
        public_key: EcdsaPublicKey,
        vaults: LazyMap<Address, (Vault, Option<Address>, bool, bool, Decimal)>, // (localStorage, bankTokenAddress, isUsingBank, localStorageIsBorrowable, loanInterest)
    }

    impl SavingsAccount {
        pub fn new(public_key: EcdsaPublicKey) -> Component {
            Self {
                public_key,
                vaults: LazyMap::new(),
            }
            .instantiate()
        }

        pub fn with_bucket(public_key: EcdsaPublicKey, bucket: Bucket) -> Component {
            let vaults = LazyMap::new();
            vaults.insert(bucket.resource_address(), (Vault::with_bucket(bucket), None, false, false, dec!("0.09")));

            Self { public_key, vaults }.instantiate()
        }

        /// Deposit a batch of buckets into this account
        pub fn deposit_batch(&mut self, buckets: Vec<Bucket>) {
            for bucket in buckets {
                self.deposit(bucket);
            }
        }

        /// Deposits resource into this account.
        pub fn deposit(&mut self, bucket: Bucket) {
            let address = bucket.resource_address();
            match self.vaults.get(&address) {
                Some( (mut vault, _bank_token_address, _is_using_bank, _local_storage_is_borrowable, _loan_interest) ) => {
                    vault.put(bucket);
                }
                None => {
                    let v = Vault::with_bucket(bucket);
                    self.vaults.insert(address, (v, None, false, false, dec!("0.09")));
                }
            }
        }

        fn non_fungible_key(&self) -> NonFungibleKey {
            NonFungibleKey::new(self.public_key.to_vec())
        }

        /// Withdraws resource from this account.
        pub fn withdraw(
            &mut self,
            amount: Decimal,
            resource_address: Address,
            account_auth: BucketRef,
        ) -> Bucket {
            account_auth.check_non_fungible_key(ECDSA_TOKEN, |key| key == &self.non_fungible_key());

            let vault = self.vaults.get(&resource_address);
            match vault {
                Some( (mut vault, _bank_token_address, _is_using_bank, _local_storage_is_borrowable, _loan_interest) ) => vault.take(amount),
                None => {
                    panic!("Insufficient balance");
                }
            }
        }

        /// Withdraws resource from this account.
        pub fn withdraw_with_auth(
            &mut self,
            amount: Decimal,
            resource_address: Address,
            auth: BucketRef,
            account_auth: BucketRef,
        ) -> Bucket {
            account_auth.check_non_fungible_key(ECDSA_TOKEN, |key| key == &self.non_fungible_key());

            let vault = self.vaults.get(&resource_address);
            match vault {
                Some( (mut vault, _bank_token_address, _is_using_bank, _local_storage_is_borrowable, _loan_interest) ) => vault.take_with_auth(amount, auth),
                None => {
                    panic!("Insufficient balance");
                }
            }
        }

        /// Withdraws non-fungibles from this account.
        pub fn withdraw_non_fungibles(
            &mut self,
            keys: BTreeSet<NonFungibleKey>,
            resource_address: Address,
            account_auth: BucketRef,
        ) -> Bucket {
            account_auth.check_non_fungible_key(ECDSA_TOKEN, |key| key == &self.non_fungible_key());

            let vault = self.vaults.get(&resource_address);
            match vault {
                Some((vault, _bank_token_address, _is_using_bank, _local_storage_is_borrowable, _loan_interest)) => {
                    let mut bucket = Bucket::new(resource_address);
                    for key in keys {
                        bucket.put(vault.take_non_fungible(&key));
                    }
                    bucket
                }
                None => {
                    panic!("Insufficient balance");
                }
            }
        }

        /// Withdraws non-fungibles from this account.
        pub fn withdraw_non_fungibles_with_auth(
            &mut self,
            keys: BTreeSet<NonFungibleKey>,
            resource_address: Address,
            auth: BucketRef,
            account_auth: BucketRef,
        ) -> Bucket {
            account_auth.check_non_fungible_key(ECDSA_TOKEN, |key| key == &self.non_fungible_key());

            let vault = self.vaults.get(&resource_address);
            match vault {
                Some((vault, _bank_token_address, _is_using_bank, _local_storage_is_borrowable, _loan_interest)) => {
                    let mut bucket = Bucket::new(resource_address);
                    for key in keys {
                        bucket.put(vault.take_non_fungible_with_auth(&key, auth.clone()));
                    }
                    bucket
                }
                None => {
                    panic!("Insufficient balance")
                }
            }
        }

        // modified flash loan code from tweeted repo
        pub fn request_loan(&mut self, amount: Decimal, resource_address: Address, component_address: Address) -> Bucket {
            let vault = self.vaults.get(&resource_address);
            match vault {
                Some((mut vault, _bank_token_address, _is_using_bank, local_storage_is_borrowable, loan_interest)) => {
                    if local_storage_is_borrowable{
                        assert!(amount < vault.amount(), "Not enough funds to loan");

                        // Call the execute method at the specified component's address with the requested funds
                        let args = vec![
                            scrypto_encode(&vault.take(amount))
                        ];

                        let mut returned_bucket: Bucket = Component::from(component_address).call::<Bucket>("execute", args).into();

                        // Make sure they repaid in loan in full
                        let amount_to_take = amount * ((loan_interest / 100) + 1);
                        assert!(returned_bucket.amount() >= amount_to_take, "You have to return more than {}", amount_to_take);

                        vault.put(returned_bucket.take(amount_to_take));

                        // Return the change back to the component
                        return returned_bucket;
                    }else{
                        panic!("vault is not allowed to be borrowed from")
                    }
                }
                None => {
                    panic!("Insufficient balance")
                }
            }
        }
    }
}
