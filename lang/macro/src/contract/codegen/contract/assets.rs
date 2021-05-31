// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
    common::GenerateCode,
    contract::{ir::Contract, SUPPORTS_ASSET_SIGNATURE},
    utils as lang_utils,
};
use cfg_if::cfg_if;
use derive_more::From;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};

#[derive(From)]
pub struct Assets<'a> {
    contract: &'a Contract,
}

impl<'a> GenerateCode for Assets<'a> {
    fn generate_code(&self) -> TokenStream2 {
        if self.contract.assets.is_empty() {
            return quote! {};
        }
        let asset_structs = self.generate_asset_structs();
        // let span = self.contract.assets.span();
        // let function_impls = self.generate_functions();
        // let constants = self.generate_constants();

        quote! {
            mod __liquid_asset {
                #[allow(unused_imports)]
                use super::*;

                #(#asset_structs)*
            }
            pub use __liquid_asset::*;
        }
    }
}

impl<'a> Assets<'a> {
    fn generate_asset_structs(&self) -> impl Iterator<Item = TokenStream2> + 'a {
        self.contract.assets.iter().map(|asset| {
            let ident = &asset.ident;
            let asset_name = asset.ident.to_string();
            let total_supply = asset.total_supply;
            let issuer = &asset.issuer;
            let span = asset.span;
            let description = asset.description.clone();
            let supports_asset_signature = SUPPORTS_ASSET_SIGNATURE;
            let call_supports_asset = if cfg!(feature = "std") {
                quote! {}
            } else {
                quote_spanned! {span =>
                    let is_contract = match liquid_lang::env::get_external_code_size(to){
                        0 => false,
                        _  => true,
                    };
                    if is_contract {
                        type Input = (String,);
                        const SUPPORTS_ASSET: liquid_primitives::Selector = {
                            let hash = liquid_primitives::hash::hash(&#supports_asset_signature.as_bytes());
                            [hash[0], hash[1], hash[2], hash[3]]
                        };
                        let mut encoded = SUPPORTS_ASSET.to_vec();
                        encoded.extend(<Input as scale::Encode>::encode(&(String::from(Self::ASSET_NAME),)));
                        match liquid_lang::env::call::<bool>(&to, &encoded) {
                            Ok(true) =>(),
                            _ => require(false, String::from("the contract doesn't know ") + Self::ASSET_NAME)
                        }
                    }
                }
            };

            if asset.fungible {
                quote_spanned! {span =>
                    #[cfg_attr(test, derive(Debug))]
                    pub struct #ident{
                        value: u64,
                        stored: bool,
                        from_self :bool,
                    }

                    impl Drop for #ident {
                        fn drop(&mut self) {
                            require(self.stored, "Asset must be deposited into an account");
                        }
                    }

                    #[allow(unused)]
                    impl<'a> #ident {
                        const TOTAL_SUPPLY : u64 = #total_supply;
                        const ISSUER : &'a str = #issuer;
                        const ASSET_NAME : &'a str = #asset_name;
                        const DESCRIPTION: &'a str = #description;
                        pub fn env(&self) -> liquid_lang::EnvAccess {
                            liquid_lang::EnvAccess {}
                        }
                        pub fn value(&self) -> u64 {
                            self.value
                        }
                        pub fn register() -> bool {
                            liquid_lang::env::register_asset(
                                Self::ASSET_NAME.as_bytes(),
                                &Self::issuer(),
                                true,
                                Self::TOTAL_SUPPLY,
                                Self::DESCRIPTION.as_bytes(),
                            )
                        }
                        pub fn total_supply() -> u64 {
                            Self::TOTAL_SUPPLY
                        }
                        pub fn issuer() -> address {
                            Self::ISSUER.parse().unwrap()
                        }
                        pub fn description() -> &'a str {
                            Self::DESCRIPTION
                        }
                        pub fn balance_of(owner: &address) -> u64 {
                            liquid_lang::env::get_asset_balance(
                                owner,
                                Self::ASSET_NAME.as_bytes(),
                            )
                        }
                        pub fn issue_to(to: &address, amount: u64) -> bool {
                            liquid_lang::env::issue_fungible_asset(
                                to,
                                Self::ASSET_NAME.as_bytes(),
                                amount,
                            )
                        }
                        pub fn withdraw_from_caller(amount: u64) -> Option<Self> {
                            let caller = liquid_lang::env::get_caller();
                            let caller_balance = Self::balance_of(&caller);
                            if caller_balance < amount {
                                return None;
                            }
                            Some(#ident {
                                value: amount,
                                stored: false,
                                from_self: false,
                            })
                        }
                        pub fn withdraw_from_self(amount: u64) -> Option<Self> {
                            let self_address = liquid_lang::env::get_address();
                            let self_balance = #ident::balance_of(&self_address);
                            if self_balance < amount {
                                return None;
                            }
                            Some(#ident {
                                value: amount,
                                stored: false,
                                from_self: true,
                            })
                        }
                        pub fn deposit(mut self, to: &address) {
                            #call_supports_asset
                            self.stored = liquid_lang::env::transfer_asset(
                                to,
                                Self::ASSET_NAME.as_bytes(),
                                self.value,
                                self.from_self,
                            );
                        }
                    }
                }
            } else {
                // not fungible token
                quote_spanned! {span =>
                    #[cfg_attr(test, derive(Debug))]
                    pub struct #ident{
                        id: u64,
                        stored: bool,
                        uri : String,
                        from_self :bool,
                    }
                    impl Drop for #ident {
                        fn drop(&mut self) {
                            require(self.stored, "Asset must be deposited into an account");
                        }
                    }
                    #[allow(unused)]
                    impl<'a> #ident{
                        const TOTAL_SUPPLY : u64 = #total_supply;
                        const ISSUER : &'a str = #issuer;
                        const ASSET_NAME : &'a str = #asset_name;
                        const DESCRIPTION: &'a str = #description;
                        #[allow(unused)]
                        pub fn env(&self) -> liquid_lang::EnvAccess {
                            liquid_lang::EnvAccess {}
                        }
                        pub fn id(&self) -> u64 {
                            self.id
                        }
                        pub fn uri(&self) -> &String {
                            &self.uri
                        }
                        pub fn register() -> bool {
                            liquid_lang::env::register_asset(
                                Self::ASSET_NAME.as_bytes(),
                                &Self::issuer(),
                                false,
                                Self::TOTAL_SUPPLY,
                                Self::DESCRIPTION.as_bytes(),
                            )
                        }
                        pub fn total_supply() -> u64 {
                            Self::TOTAL_SUPPLY
                        }
                        pub fn issuer() -> address {
                            Self::ISSUER.parse().unwrap()
                        }
                        pub fn description() -> &'a str {
                            Self::DESCRIPTION
                        }
                        pub fn balance_of(owner: &address) -> u64 {
                            liquid_lang::env::get_asset_balance(
                                owner,
                                Self::ASSET_NAME.as_bytes(),
                            )
                        }
                        pub fn tokens_of(owner: &address) -> Vec<u64> {
                            liquid_lang::env::get_not_fungible_asset_ids(
                                owner,
                                Self::ASSET_NAME.as_bytes(),
                            )
                        }
                        pub fn issue_to(to: &address, uri: &str) -> Option<u64> {
                            match liquid_lang::env::issue_not_fungible_asset(
                                to,
                                Self::ASSET_NAME.as_bytes(),
                                uri.as_bytes(),
                            ){
                                0 => None,
                                v => Some(v),
                            }
                        }
                        pub fn withdraw_from_caller(id: u64) -> Option<Self> {
                            let caller = liquid_lang::env::get_caller();
                            let uri = liquid_lang::env::get_not_fungible_asset_info(
                                &caller,
                                Self::ASSET_NAME.as_bytes(),
                                id,
                            );
                            if uri.is_empty() {
                                return None;
                            }
                            Some(#ident {
                                id,
                                stored: false,
                                uri,
                                from_self: false,
                            })
                        }
                        pub fn withdraw_from_self(id: u64) -> Option<Self> {
                            let self_address = liquid_lang::env::get_address();
                            let uri = liquid_lang::env::get_not_fungible_asset_info(
                                &self_address,
                                Self::ASSET_NAME.as_bytes(),
                                id,
                            );
                            if uri.is_empty() {
                                return None;
                            }
                            Some(#ident {
                                id,
                                stored: false,
                                uri,
                                from_self: true,
                            })
                        }
                        pub fn deposit(mut self, to: &address) {
                            #call_supports_asset
                            self.stored = liquid_lang::env::transfer_asset(
                                to,
                                Self::ASSET_NAME.as_bytes(),
                                self.id,
                                self.from_self,
                            );
                        }
                    }
                }
            }
        })
    }
}
