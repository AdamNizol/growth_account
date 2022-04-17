# Scrypto Growth-Account
 
## Advantages of a Growth-Account
- Tokens can be toggled between earning passive interest or remaining in the account
- While earning interest, tokens can still be spent from the account as usual
<br>

## Setup a Growth-Account in Resim
```
resim reset
resim new-account
```
save the ***$pubkey*** and ***$account***  
<br>
```
resim show $account
```
save the xrd address as ***$xrd***  
<br>
```
resim publish .
```
save the ***$package***  
<br>
```
resim call-function $package Bank new <loan fee> <bank fee>
```
>**\<loan fee\>:** recommended: 0.09  - the percentage fee on flashloans  
>**\<bank fee\>:** recommended: 5     - the percentage of loan profits returned to the bank  

save the ***$component***  
<br>
```
resim call-function $package SavingsAccount with_bucket $pubkey 1000000,$xrd $component
```
save the component address as ***$account***
<br>
```
resim set-default-account $account $pubkey
```
<br>

## Toggling passive growth
To allow a token to be banked and earning interest you need to call the **bank_token(token Address)** method on your account. 
Unfortunately since this requires account auth like the withdraw method, it cannot be run directly with resim but you can use a rtm file:
```
CLONE_BUCKET_REF BucketRef(1u32) BucketRef("badge1");
CALL_METHOD Address("<account address>") "bank_token" Address("<token address>") BucketRef("badge1");
```
save this as "banktoken.rtm"  
>**\<account address\>:** the address for your account  
>**\<token address\>:** the address for the token you want to bank  
<br>
Finally you can run

```
resim run banktoken.rtm
```
<br>

> ### To unbank a token follow the same steps but run the "unbank_token" method instead of "bank_token"
