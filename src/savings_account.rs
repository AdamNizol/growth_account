use scrypto::prelude::*;

blueprint! {
    struct SavingsAccount {
        public_key: EcdsaPublicKey,
        vaults: LazyMap<Address, (Vault, Option<Address>, bool, bool, Decimal)>, // (localStorage, bankTokenAddress, isUsingBank, localStorageIsBorrowable, loanInterest)
        bank: Address,
    }

    impl SavingsAccount {
        // TODO: at 0.4.0 revert to:
        // pub fn new(public_key: EcdsaPublicKey) -> Component {
        pub fn new(public_key: String, bank: Address) -> Component {
            Self {
                // TODO: at 0.4.0 revert to:
                // public_key,
                public_key: EcdsaPublicKey::from_str(public_key.as_str()).unwrap(),
                vaults: LazyMap::new(),
                bank
            }
            .instantiate()
        }

        // TODO: at 0.4.0 revert to:
        // pub fn with_bucket(public_key: EcdsaPublicKey, bucket: Bucket) -> Component {
        pub fn with_bucket(public_key: String, bucket: Bucket, bank: Address) -> Component {
            let vaults = LazyMap::new();
            vaults.insert(bucket.resource_address(), (Vault::with_bucket(bucket), None, false, false, dec!("0.09")));

            Self {
                // TODO: at 0.4.0 revert to:
                // public_key,
                public_key: EcdsaPublicKey::from_str(public_key.as_str()).unwrap(),
                vaults,
                bank
            }.instantiate()
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
                Some( (mut vault, bank_token_address, is_using_bank, _local_storage_is_borrowable, _loan_interest) ) => {
                    if is_using_bank {
                        let (mut v, _bta, _iub, _lsb, _li) = self.vaults.get(&bank_token_address.unwrap()).unwrap();
                        let args = vec![
                            scrypto_encode(&bucket)
                        ];
                        let lended_tokens: Bucket = Component::from(self.bank).call::<Bucket>("deposit", args).into();
                        v.put(lended_tokens);
                    }else{
                        vault.put(bucket);
                    }
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
                Some( (mut vault, bank_token_address, is_using_bank, _local_storage_is_borrowable, _loan_interest) ) => {
                    if is_using_bank {
                        let (mut v, _bta, _iub, _lsb, _li) = self.vaults.get(&bank_token_address.unwrap()).unwrap();
                        let args = vec![
                            scrypto_encode(&v.take(v.amount()))
                        ];
                        let mut base_tokens: Bucket = Component::from(self.bank).call::<Bucket>("withdraw", args).into();
                        let withdrawn_tokens = base_tokens.take(amount);
                        let lended_tokens: Bucket = Component::from(self.bank).call::<Bucket>("deposit", vec![scrypto_encode(&base_tokens)]).into();
                        v.put(lended_tokens);
                        return withdrawn_tokens;
                    }else{
                        vault.take(amount)
                    }
                }
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

        // makes a token auto-lended
        pub fn bank_token(
            &mut self,
            resource_address: Address,
            account_auth: BucketRef,
        ) -> () {
            account_auth.check_non_fungible_key(ECDSA_TOKEN, |key| key == &self.non_fungible_key());

            let vault = self.vaults.get(&resource_address);
            match vault {
                Some((mut vault, _bank_token_address, is_using_bank, local_storage_is_borrowable, loan_interest)) => {
                    assert!(!is_using_bank, "already using Bank for this token");
                    let args = vec![
                        scrypto_encode(&vault.take(vault.amount()))
                    ];

                    let lended_tokens: Bucket = Component::from(self.bank).call::<Bucket>("deposit", args).into();
                    self.vaults.insert(resource_address, (vault, Some(lended_tokens.resource_address()), true, local_storage_is_borrowable, loan_interest) );
                    self.deposit(lended_tokens);
                }
                None => {
                    panic!("Insufficient balance")
                }
            }
        }

        // makes a token no longer auto-lended
        pub fn unbank_token(
            &mut self,
            resource_address: Address,
            account_auth: BucketRef,
        ) -> () {
            account_auth.check_non_fungible_key(ECDSA_TOKEN, |key| key == &self.non_fungible_key());

            let vault = self.vaults.get(&resource_address);
            match vault {
                Some((mut vault, bank_token_address, is_using_bank, local_storage_is_borrowable, loan_interest)) => {
                    assert!(is_using_bank, "Bank is not currently used for this token");

                    let (mut v, _bta, _iub, _lsb, _li) = self.vaults.get(&bank_token_address.unwrap()).unwrap();
                    let args = vec![
                        scrypto_encode(&v.take(v.amount()))
                    ];

                    let base_tokens: Bucket = Component::from(self.bank).call::<Bucket>("withdraw", args).into();
                    vault.put(base_tokens);
                    self.vaults.insert(resource_address, (vault, bank_token_address, false, local_storage_is_borrowable, loan_interest) );
                }
                None => {
                    panic!("Insufficient balance")
                }
            }
        }

        pub fn set_borrowable(
            &mut self,
            resource_address: Address,
            account_auth: BucketRef,
        ) -> () {
            account_auth.check_non_fungible_key(ECDSA_TOKEN, |key| key == &self.non_fungible_key());

            let (vault, bank_token_address, is_using_bank, local_storage_is_borrowable, loan_interest) = self.vaults.get(&resource_address).unwrap();
            assert!(!local_storage_is_borrowable, "That loken is already borrowable");
            self.vaults.insert(resource_address, (vault, bank_token_address, is_using_bank, true, loan_interest) );
        }

        pub fn set_unborrowable(
            &mut self,
            resource_address: Address,
            account_auth: BucketRef,
        ) -> () {
            account_auth.check_non_fungible_key(ECDSA_TOKEN, |key| key == &self.non_fungible_key());

            let (vault, bank_token_address, is_using_bank, local_storage_is_borrowable, loan_interest) = self.vaults.get(&resource_address).unwrap();
            assert!(local_storage_is_borrowable, "That loken is already borrowable");
            self.vaults.insert(resource_address, (vault, bank_token_address, is_using_bank, false, loan_interest) );
        }

        pub fn set_interest_rate(
            &mut self,
            resource_address: Address,
            interest_rate: Decimal,
            account_auth: BucketRef,
        ) -> () {
            account_auth.check_non_fungible_key(ECDSA_TOKEN, |key| key == &self.non_fungible_key());

            let (vault, bank_token_address, is_using_bank, local_storage_is_borrowable, _loan_interest) = self.vaults.get(&resource_address).unwrap();
            self.vaults.insert(resource_address, (vault, bank_token_address, is_using_bank, local_storage_is_borrowable, interest_rate) );
        }

        // modified flash loan code from tweeted repo
        pub fn request_loan(&mut self, amount: Decimal, resource_address: Address, component_address: Address) -> Bucket {
            let vault = self.vaults.get(&resource_address);
            match vault {
                Some((mut vault, _bank_token_address, _is_using_bank, local_storage_is_borrowable, loan_interest)) => {
                    if local_storage_is_borrowable{
                        assert!(amount <= vault.amount(), "Not enough funds to loan");

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

        // lends out all the cash in the vault
        pub fn request_max_loan(&mut self, currency: Address, component_address: Address) -> Bucket {
            match self.vaults.get(&currency){
                Some((vault, _bank_token_address, _is_using_bank, _local_storage_is_borrowable, _loan_interest)) =>{
                    self.request_loan(vault.amount(), currency, component_address)
                }
                None =>{
                    panic!("No liquidity for this token is available")
                }
            }
        }

    }
}
