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

mod codegen;
pub mod ir;

use crate::{common::GenerateCode, error::*, utils::check_idents};
use proc_macro2::TokenStream as TokenStream2;
use std::{cell::RefCell, convert::TryFrom};
use syn::{spanned::Spanned, Result};

pub enum GenerateMode {
    Contract,
    Interface,
}

pub fn generate(
    attr: TokenStream2,
    input: TokenStream2,
    mode: GenerateMode,
) -> TokenStream2 {
    match generate_impl(attr, input, mode) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

// It seems that rls may make compiler working in multi-threaded mode to acquire
// intelli sense information. If this static variable not defined as thread local, the
// rls will display the error that never exits...
thread_local! {
    pub static CONTRACT_DEFINITION_COUNT: RefCell<u8> = RefCell::new(0);
}

fn generate_impl(
    attr: TokenStream2,
    input: TokenStream2,
    mode: GenerateMode,
) -> Result<TokenStream2> {
    check_idents(input.clone())?;

    match mode {
        GenerateMode::Contract => {
            let params = syn::parse2::<ir::ContractParams>(attr)?;
            let input_span = input.span();
            let item_mod = syn::parse2::<syn::ItemMod>(input)?;

            let had_redefined = CONTRACT_DEFINITION_COUNT.with(|def_count| {
                let count = *def_count.borrow();
                if count == 0 {
                    *def_count.borrow_mut() = count + 1;
                    false
                } else {
                    true
                }
            });

            if had_redefined {
                bail_span!(
                    input_span,
                    "contract `{}` redefined here, the project should only contain 1 \
                     contract definition",
                    item_mod.ident
                );
            }

            let liquid_ir = ir::Contract::try_from((params, item_mod))?;
            Ok(liquid_ir.generate_code())
        }
        GenerateMode::Interface => {
            let params = syn::parse2::<ir::InterfaceParams>(attr)?;
            let item_mod = syn::parse2::<syn::ItemMod>(input)?;
            let liquid_ir = ir::Interface::try_from((params, item_mod))?;
            Ok(liquid_ir.generate_code())
        }
    }
}

pub const SUPPORTS_ASSET_NAME: &str = "__liquid_supports_asset";
pub const SUPPORTS_ASSET_SIGNATURE: &str = "__liquid_supports_asset(string)";
