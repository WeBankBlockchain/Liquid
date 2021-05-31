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

use crate::{common::GenerateCode, contract::ir::Contract, utils as lang_utils};
use derive_more::From;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

#[derive(From)]
pub struct Events<'a> {
    contract: &'a Contract,
}

impl<'a> GenerateCode for Events<'a> {
    fn generate_code(&self) -> TokenStream2 {
        if self.contract.events.is_empty() {
            return quote! {};
        }

        let event_enum = self.generate_event_enum();
        let topics_impls = self.generate_topics_impls();
        let emit_trait = self.generate_emit_trait();

        quote! {
            mod __liquid_event {
                #[allow(unused_imports)]
                use super::*;

                #(#topics_impls)*
                #event_enum
                #emit_trait
            }

            pub use __liquid_event::{Event, Emit};
        }
    }
}

impl<'a> Events<'a> {
    fn generate_emit_trait(&self) -> TokenStream2 {
        quote! {
            pub trait Emit {
                type Event;

                fn emit<E>(self, event: E)
                where
                    E: Into<Self::Event>;
            }

            impl Emit for liquid_lang::EnvAccess {
                type Event = Event;

                fn emit<E>(self, event: E)
                where
                    E: Into<Self::Event>
                {
                    liquid_lang::env::emit(event.into())
                }
            }
        }
    }

    fn generate_event_enum(&self) -> TokenStream2 {
        let event_idents = self
            .contract
            .events
            .iter()
            .map(|item_event| &item_event.ident)
            .collect::<Vec<_>>();

        quote! {
            pub enum Event {
                #(#event_idents(#event_idents),)*
            }

            #(
                impl From<#event_idents> for Event {
                    fn from(event: #event_idents) -> Self {
                        Event::#event_idents(event)
                    }
                }
            )*

            impl liquid_primitives::Topics for Event {
                fn topics(&self) -> liquid_prelude::vec::Vec<liquid_primitives::types::Hash> {
                    match self {
                        #(
                            Event::#event_idents(event) => event.topics(),
                        )*
                    }
                }
            }

            impl scale::Encode for Event {
                fn encode(&self) -> Vec<u8> {
                    match self {
                        #(
                            Event::#event_idents(event) => event.encode(),
                        )*
                    }
                }
            }
        }
    }

    fn generate_topics_impls(&'a self) -> impl Iterator<Item = TokenStream2> + 'a {
        self.contract.events.iter().map(move |item_event| {
            let span = item_event.span;
            let event_ident = &item_event.ident;
            let event_fields = &item_event.fields;
            let event_name = event_ident.to_string();
            let event_name_bytes = event_name.as_bytes();
            let event_field_tys = event_fields.iter().enumerate().map(|(i, field)| {
                let ty = &field.ty;
                if !item_event.indexed_fields.iter().any(|index| *index == i) {
                    quote_spanned! { ty.span() =>
                        <#ty as liquid_lang::You_Should_Use_An_Valid_Output_Type>::T
                    }
                } else {
                    quote_spanned! { ty.span() =>
                        <#ty as liquid_lang::You_Should_Use_An_Valid_Topic_Type>::T
                    }
                }
            }).collect::<Vec<_>>();

            let sig_hash = quote_spanned! { span =>
                {
                    #[allow(non_camel_case_types)]
                    struct __LIQUID_EVENT_FIELDS_CHECKER(#(#event_field_tys,)*);
                    liquid_primitives::hash::hash(&[#(#event_name_bytes),*]).into()
                }
            };

            let topic_hash = {
                let calculate_topics = item_event.indexed_fields.iter().map(|index| {
                    let ident = &event_fields[*index].ident;
                    let ty = &event_fields[*index].ty;
                    quote_spanned! { ty.span() =>
                        <#ty as liquid_lang::You_Should_Use_An_Valid_Topic_Type>::topic(&self.#ident)
                    }
                });

                quote! {
                    #(
                        #calculate_topics,
                    )*
                }
            };

            let impls = quote_spanned! { span =>
                impl liquid_primitives::Topics for #event_ident {
                    fn topics(&self) -> liquid_prelude::vec::Vec<liquid_primitives::types::Hash> {
                        [#sig_hash, #topic_hash].to_vec()
                    }
                }
            };

            impls
        })
    }
}

#[derive(From)]
pub struct EventStructs<'a> {
    contract: &'a Contract,
}

impl<'a> GenerateCode for EventStructs<'a> {
    fn generate_code(&self) -> TokenStream2 {
        if self.contract.events.is_empty() {
            return quote! {};
        }

        let event_struts = self.generate_event_structs();

        quote! {
            #(#event_struts)*
        }
    }
}

impl<'a> EventStructs<'a> {
    fn generate_event_structs(&'a self) -> impl Iterator<Item = TokenStream2> + 'a {
        self.contract.events.iter().map(move |item_event| {
            let span = item_event.span;
            let ident = &item_event.ident;
            let attrs = lang_utils::filter_non_liquid_attributes(&item_event.attrs);
            let mut fields = item_event.fields.clone();
            fields.iter_mut().for_each(|field| {
                field.vis = syn::Visibility::Public(syn::VisPublic {
                    pub_token: Default::default(),
                });
                field
                    .attrs
                    .retain(|attr| !lang_utils::is_liquid_attribute(attr));
            });
            let fields = fields.iter().enumerate().map(|(i, field)| {
                if item_event.indexed_fields.contains(&i) {
                    quote! {
                        #[codec(skip)]
                        #field
                    }
                } else {
                    quote! {
                        #field
                    }
                }
            });

            quote_spanned!(span =>
                #(#attrs)*
                #[derive(scale::Encode)]
                pub struct #ident {
                    #(#fields,)*
                }
            )
        })
    }
}
