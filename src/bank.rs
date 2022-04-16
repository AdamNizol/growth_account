use scrypto::prelude::*;

blueprint! {
    struct Bank {
        loan_interest: Decimal,
        lender_badge: Vault,
        lender_accounts: LazyMap<Address, (Vault, ResourceDef)>,
        lender_lookup: LazyMap<Address, Address>, // <LenderTokenAddr, TokenAddr>
    }

    impl Bank {

        pub fn new(loan_interest: Decimal) -> Component {
            let admin_badge: Bucket = ResourceBuilder::new_fungible(DIVISIBILITY_NONE).initial_supply_fungible(1);

            Self {
                loan_interest: loan_interest,
                lender_badge: Vault::with_bucket(admin_badge),
                lender_accounts: LazyMap::new(),
                lender_lookup: LazyMap::new()
            }
            .instantiate()
        }

        // mints new lender tokens at the current exchange rate
        pub fn deposit(&mut self, payment: Bucket) -> Bucket {
            let address = payment.resource_address();
            match self.lender_accounts.get(&address){
                Some(acc) =>{
                    let (mut vault, mut resource) = acc;
                    let exchange_rate: Decimal = if vault.amount() > dec!(0) { resource.total_supply()/vault.amount() } else { dec!(1) };
                    let lenders_bought: Decimal = exchange_rate*payment.amount();
                    vault.put(payment);
                    self.lender_badge.authorize(|auth|{
                        resource.mint(lenders_bought, auth)
                    })
                }
                None =>{
                    let token_meta = ResourceDef::from(payment.resource_address()).metadata();
                    let name = if token_meta.contains_key("name"){format!("L-{}",token_meta["name"])}else{"LenderToken".to_string()};
                    let symbol = if token_meta.contains_key("name"){format!("L-{}",token_meta["symbol"])}else{"LT".to_string()};
                    let v = Vault::with_bucket(payment);
                    let mut lender_resource_def: ResourceDef = ResourceBuilder::new_fungible(DIVISIBILITY_MAXIMUM)
                        .metadata("name", name)
                        .metadata("symbol", symbol)
                        .flags(MINTABLE | BURNABLE)
                        .badge(self.lender_badge.resource_def(), MAY_MINT | MAY_BURN)
                        .metadata("description", "A lender token")
                        .no_initial_supply();
                    let t = self.lender_badge.authorize(|auth|{
                        lender_resource_def.mint(v.amount(), auth)
                    });
                    self.lender_accounts.insert(address, (v,lender_resource_def));
                    self.lender_lookup.insert(t.resource_address(),address);
                    return t;
                }
            }
        }

        pub fn withdraw(&mut self, lenders: Bucket) -> Bucket {
            let lender_address = lenders.resource_address();
            let address = match self.lender_lookup.get(&lender_address){
                Some(addr) => {addr}
                None => {panic!("Invalid lender token")}
            };
            match self.lender_accounts.get(&address){
                Some(acc) =>{
                    let (mut vault, mut resource) = acc;
                    let cash_returned: Decimal = (vault.amount()/resource.total_supply())*lenders.amount();
                    self.lender_badge.authorize(|auth|{
                        resource.burn_with_auth(lenders, auth);
                    });
                    vault.take(cash_returned)
                }
                None => {
                    panic!("No lender account found")
                }
            }
        }

        // modified flash loan code from tweeted repo
        pub fn request_loan(&mut self, amount: Decimal, currency: Address, component_address: Address) -> Bucket {
            match self.lender_accounts.get(&currency){
                Some(acc) =>{
                    let (mut vault, _resource) = acc;

                    assert!(amount < vault.amount(), "Not enough funds to loan");

                    // Call the execute method at the specified component's address with the requested funds
                    let args = vec![
                        scrypto_encode(&vault.take(amount))
                    ];

                    let mut returned_bucket: Bucket = Component::from(component_address).call::<Bucket>("execute", args).into();

                    // Make sure they repaid in loan in full
                    let amount_to_take = amount * ((self.loan_interest / 100) + 1);
                    assert!(returned_bucket.amount() >= amount_to_take, "You have to return more than {}", amount_to_take);

                    vault.put(returned_bucket.take(amount_to_take));

                    // Return the change back to the component
                    return returned_bucket;
                }
                None => {
                    panic!("No liquidity for this token is available")
                }
            }
        }

        // lends out all the cash in the vault
        pub fn request_max_loan(&mut self, currency: Address, component_address: Address) -> Bucket {
            match self.lender_accounts.get(&currency){
                Some(acc) =>{
                    let (vault, _resource) = acc;
                    self.request_loan(vault.amount(), currency, component_address)
                }
                None =>{
                    panic!("No liquidity for this token is available")
                }
            }
        }

        // similar to request_loan but will loan max rather than fail if the amount cannot be filled
        pub fn request_loan_upto(&mut self, amount: Decimal, currency: Address, component_address: Address) -> Bucket {
            match self.lender_accounts.get(&currency){
                Some(acc) =>{
                    let (vault, _resource) = acc;
                    self.request_loan(if amount > vault.amount(){vault.amount()}else{amount}, currency, component_address)
                }
                None =>{
                    panic!("No liquidity for this token is available")
                }
            }
        }

        pub fn get_balance(&self, currency: Address) -> Decimal {
            match self.lender_accounts.get(&currency){
                Some(acc) =>{
                    let (vault, _resource) = acc;
                    vault.amount()
                }
                None =>{
                    panic!("No liquidity for this token is available")
                }
            }
        }

    }
}
