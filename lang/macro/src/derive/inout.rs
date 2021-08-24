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

use liquid_prelude::vec::Vec;
use proc_macro2::{Ident, Literal, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{self, parse::Result, spanned::Spanned, Data, DeriveInput, Fields, Type};

pub fn generate(input: TokenStream2) -> TokenStream2 {
    match generate_impl(input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error(),
    }
}

fn generate_impl(input: TokenStream2) -> Result<TokenStream2> {
    let ast: DeriveInput = syn::parse2(input)?;

    match &ast.vis {
        syn::Visibility::Public(_) => (),
        _ => bail!(ast, "the visibility of this type should be `pub`"),
    }

    if ast.generics.type_params().count() > 0 {
        bail!(&ast.generics, "generic are not supported")
    }

    let ident = &ast.ident;
    let mut shadow = match &ast.data {
        Data::Union(ref union_data) => {
            bail!(&union_data.union_token, "unions are not supported")
        }
        Data::Struct(ref struct_data) => {
            let fields = &struct_data.fields;
            let mut is_unnamed = true;

            let (field_names, field_tys): (Vec<_>, Vec<_>) = match fields {
                Fields::Named(fields_named) => {
                    is_unnamed = false;
                    fields_named
                        .named
                        .iter()
                        .map(|field| (field.ident.as_ref().unwrap().clone(), &field.ty))
                        .unzip()
                }
                Fields::Unnamed(fields_unnamed) => fields_unnamed
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, field)| {
                        (Ident::new(&format!("_{}", i), field.span()), &field.ty)
                    })
                    .unzip(),
                Fields::Unit => (Vec::new(), Vec::new()),
            };

            let encode_shadow_struct = generate_encode_shadow_struct(
                ident,
                &field_names,
                &field_tys,
                is_unnamed,
            );
            let decode_shadow_struct = generate_decode_shadow_struct(
                ident,
                &field_names,
                &field_tys,
                is_unnamed,
            );
            let abi_impls = generate_abi_struct(
                if is_unnamed {
                    None
                } else {
                    Some(field_names.as_slice())
                },
                &field_tys,
                ident,
            );

            let mut field_checkers = Vec::new();
            for (i, ty) in field_tys.iter().enumerate() {
                let field_checker = Ident::new(
                    &format!("__LIQUID_FIELD_CHECKER_{}", i),
                    Span::call_site(),
                );
                field_checkers.push(quote_spanned! { ty.span() =>
                    #[allow(non_camel_case_types)]
                    struct #field_checker(<#ty as liquid_lang::You_Should_Use_An_Valid_Input_Type>::T, <#ty as liquid_lang::You_Should_Use_An_Valid_Output_Type>::T);
                })
            }

            quote! {
                #(#field_checkers)*
                #encode_shadow_struct
                #decode_shadow_struct
                #abi_impls
            }
        }
        Data::Enum(ref enum_data) => {
            if enum_data.variants.is_empty() {
                bail!(ast, "empty enum is not supported")
            }

            let mut variant_count = 0;
            for variant in &enum_data.variants {
                variant_count += 1;
                if variant.discriminant.is_some() {
                    bail!(variant, "custom discriminant is not supported")
                }
            }

            if variant_count > 256 {
                bail!(ast, "only enums with at most 256 variants are derivable")
            }

            let mut field_checkers = Vec::new();
            let variants = enum_data
                .variants
                .iter()
                .map(|variant| {
                    let ident = &variant.ident;
                    let fields = &variant.fields;
                    let mut unnamed = true;
                    let mut is_unit = false;

                    let (field_names, field_tys): (Vec<_>, Vec<_>) = match fields {
                        Fields::Named(fields_named) => {
                            unnamed = false;

                            fields_named
                                .named
                                .iter()
                                .map(|field| {
                                    let field_checker = Ident::new(
                                        &format!("__LIQUID_FIELD_CHECKER_{}", field_checkers.len()),
                                        Span::call_site(),
                                    );
                                    let ty = &field.ty;
                                    field_checkers.push(quote_spanned! { ty.span() =>
                                        #[allow(non_camel_case_types)]
                                        struct #field_checker(<#ty as liquid_lang::You_Should_Use_An_Valid_Input_Type>::T, <#ty as liquid_lang::You_Should_Use_An_Valid_Output_Type>::T);
                                    });

                                    (field.ident.as_ref().unwrap().clone(), ty)
                                })
                                .unzip()
                        }
                        Fields::Unnamed(fields_unnamed) => fields_unnamed
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, field)| {
                                let field_checker = Ident::new(
                                    &format!("__LIQUID_FIELD_CHECKER_{}", field_checkers.len()),
                                    Span::call_site(),
                                );
                                let ty = &field.ty;
                                field_checkers.push(quote_spanned! { ty.span() =>
                                    #[allow(non_camel_case_types)]
                                    struct #field_checker(<#ty as liquid_lang::You_Should_Use_An_Valid_Input_Type>::T, <#ty as liquid_lang::You_Should_Use_An_Valid_Output_Type>::T);
                                });

                                (Ident::new(&format!("_{}", i), field.span()), ty)
                            })
                            .unzip(),
                        Fields::Unit => {
                            is_unit = true;
                            (Vec::new(), Vec::new())
                        },
                    };

                    Variant {
                        ident,
                        unnamed,
                        is_unit,
                        field_names,
                        field_tys,
                    }
                })
                .collect::<Vec<_>>();

            let encode_shadow_enum = generate_encode_shadow_enum(ident, variants.iter());
            let decode_shadow_enum = generate_decode_shadow_enum(ident, variants.iter());
            let abi_impls = generate_abi_enum(ident, variants.as_slice());

            quote! {
                #(#field_checkers)*
                #encode_shadow_enum
                #decode_shadow_enum
                #abi_impls
            }
        }
    };

    shadow.extend(quote! {
        impl liquid_lang::You_Should_Use_An_Valid_Input_Type for #ident {}
        impl liquid_lang::You_Should_Use_An_Valid_Output_Type for #ident {}
    });

    if cfg!(feature = "contract") {
        shadow.extend(quote! {
            impl liquid_lang::You_Should_Use_An_Valid_State_Type for #ident {}
        });
    }
    Ok(shadow)
}

struct Variant<'a> {
    ident: &'a Ident,
    unnamed: bool,
    is_unit: bool,
    field_names: Vec<Ident>,
    field_tys: Vec<&'a Type>,
}

fn generate_encode_shadow_struct(
    ident: &Ident,
    field_names: &[Ident],
    field_tys: &[&Type],
    is_unnamed: bool,
) -> TokenStream2 {
    let fields = field_names.iter().enumerate().map(|(i, field_name)| {
        let field_ty = field_tys[i];
        quote!(#field_name: &'a #field_ty,)
    });
    let assigns = if is_unnamed {
        field_names
            .iter()
            .enumerate()
            .map(|(i, field_name)| {
                let field = Literal::usize_unsuffixed(i);
                quote!(#field_name: &origin.#field,)
            })
            .collect::<Vec<_>>()
    } else {
        field_names
            .iter()
            .map(|field_name| quote!(#field_name: &origin.#field_name,))
            .collect::<Vec<_>>()
    };

    quote! {
        #[derive(scale::Encode)]
        struct EncodeShadow<'a> {
            #(#fields)*
            #[codec(skip)]
            _marker: core::marker::PhantomData<&'a ()>,
        }

        impl<'a> From<&'a #ident> for EncodeShadow<'a> {
            fn from(origin: &'a #ident) -> Self {
                Self {
                    #(#assigns)*
                    _marker: Default::default(),
                }
            }
        }

        impl scale::Encode for #ident {
            fn encode(&self) -> Vec<u8> {
                let encode_shadow: EncodeShadow::<'_> = self.into();
                encode_shadow.encode()
            }
        }
    }
}

fn generate_encode_shadow_enum<'a>(
    ident: &Ident,
    variants: impl Iterator<Item = &'a Variant<'a>>,
) -> TokenStream2 {
    let (new_variants, arms): (Vec<_>, Vec<_>) = variants
        .map(|variant| {
            let variant_ident = variant.ident;
            let field_names = &variant.field_names;
            let field_tys = &variant.field_tys;
            let fields =
                field_names
                    .iter()
                    .zip(field_tys.iter())
                    .map(|(field_name, field_ty)| {
                        quote! {
                            #field_name: &'a #field_ty
                        }
                    });
            let new_variant = quote! {
                #variant_ident {
                    #(#fields,)*
                },
            };

            let arms = if variant.is_unit {
                quote! {
                    #ident::#variant_ident => Self::#variant_ident{},
                }
            } else {
                let ref_fields = field_names
                    .iter()
                    .map(|field_name| {
                        quote! {
                            ref #field_name
                        }
                    })
                    .collect::<Vec<_>>();

                if variant.unnamed {
                    quote! {
                        #ident::#variant_ident(#(#ref_fields,)*) => Self::#variant_ident {
                            #(#field_names,)*
                        },
                    }
                } else {
                    quote! {
                        #ident::#variant_ident{#(#ref_fields,)*} => Self::#variant_ident {
                            #(#field_names,)*
                        },
                    }
                }
            };
            (new_variant, arms)
        })
        .unzip();

    quote! {
        #[derive(scale::Encode)]
        enum EncodeShadow<'a> {
            #(#new_variants)*
        }

        impl<'a> From<&'a #ident> for EncodeShadow<'a> {
            fn from(origin: &'a #ident) -> Self {
                match origin {
                    #(#arms)*
                }
            }
        }

        impl scale::Encode for #ident {
            fn encode(&self) -> Vec<u8> {
                let encode_shadow: EncodeShadow::<'_> = self.into();
                encode_shadow.encode()
            }
        }
    }
}

fn generate_decode_shadow_struct(
    ident: &Ident,
    field_names: &[Ident],
    field_tys: &[&Type],
    is_unnamed: bool,
) -> TokenStream2 {
    let fields = field_names.iter().enumerate().map(|(i, field_name)| {
        let field_ty = field_tys[i];
        quote!(#field_name: #field_ty,)
    });
    let assigns = if is_unnamed {
        field_names
            .iter()
            .map(|field_name| quote!(origin.#field_name,))
            .collect::<Vec<_>>()
    } else {
        field_names
            .iter()
            .map(|field_name| quote!(#field_name: origin.#field_name,))
            .collect::<Vec<_>>()
    };
    let create_self = if is_unnamed {
        quote! { Self ( #(#assigns)* ) }
    } else {
        quote! { Self { #(#assigns)* } }
    };

    quote! {
        #[derive(scale::Decode)]
        struct DecodeShadow {
            #(#fields)*
        }

        impl scale::Decode for #ident {
            fn decode<I: scale::Input>(value: &mut I) -> ::core::result::Result<Self, scale::Error> {
                let origin = <DecodeShadow as scale::Decode>::decode(value)?;
                Ok(#create_self)
            }
        }
    }
}

fn generate_decode_shadow_enum<'a>(
    ident: &Ident,
    variants: impl Iterator<Item = &'a Variant<'a>>,
) -> TokenStream2 {
    let (new_variants, arms): (Vec<_>, Vec<_>) = variants.map(|variant| {
        let variant_ident = variant.ident;
        let field_names = &variant.field_names;
        let field_tys = &variant.field_tys;
        let fields =
            field_names
                .iter()
                .zip(field_tys.iter())
                .map(|(field_name, field_ty)| {
                    quote! {
                        #field_name: #field_ty
                    }
                });
        let new_variants = quote! {
            #variant_ident {
                #(#fields,)*
            },
        };

        let arms = if variant.is_unit {
            debug_assert!(field_names.is_empty());
            quote! {
                DecodeShadow::#variant_ident{} => Ok(#ident::#variant_ident),
            }
        } else if variant.unnamed {
                quote! {
                    DecodeShadow::#variant_ident{#(#field_names,)*} => Ok(#ident::#variant_ident (
                        #(#field_names,)*
                    )),
                }
        } else {
            quote! {
                DecodeShadow::#variant_ident{#(#field_names,)*} => Ok(#ident::#variant_ident {
                    #(#field_names,)*
                }),
            }
        };
        (new_variants, arms)
    }).unzip();

    quote! {
        #[derive(scale::Decode)]
        enum DecodeShadow {
            #(#new_variants)*
        }

        impl scale::Decode for #ident {
            fn decode<I: scale::Input>(value: &mut I) -> ::core::result::Result<Self, scale::Error> {
                let origin = <DecodeShadow as scale::Decode>::decode(value)?;
                match origin {
                    #(#arms)*
                }
            }
        }
    }
}

fn generate_abi_struct(
    field_names: Option<&[Ident]>,
    field_tys: &[&syn::Type],
    ident: &Ident,
) -> TokenStream2 {
    let field_param_abis = if let Some(field_names) = field_names {
        field_names
        .iter()
        .map(|name| name.to_string())
        .zip(field_tys.iter())
        .map(|(field_name, field_ty)| {
            quote! {
                <#field_ty as liquid_abi_gen::traits::GenerateParamAbi>::generate_param_abi(#field_name.to_owned())
            }
        }).collect::<Vec<_>>()
    } else {
        field_tys.iter().map(|field_ty| {
            quote! {
                <#field_ty as liquid_abi_gen::traits::GenerateParamAbi>::generate_param_abi("".into())
            }
        }).collect::<Vec<_>>()
    };

    let ident_str = ident.to_string();
    quote! {
        #[cfg(feature = "liquid-abi-gen")]
        impl liquid_abi_gen::traits::GenerateParamAbi for #ident {
            fn generate_ty_name() -> liquid_prelude::string::String {
                String::from("tuple")
            }

            fn generate_internal_ty_name() -> liquid_prelude::string::String {
                let mut internal_ty_name = String::from("struct.");
                internal_ty_name.push_str(#ident_str);
                internal_ty_name
            }

            fn generate_param_abi(name: String) -> liquid_abi_gen::ParamAbi {
                let mut components = Vec::new();
                #(components.push(#field_param_abis);)*
                liquid_abi_gen::ParamAbi::Composite(
                    liquid_abi_gen::CompositeAbi {
                        trivial: liquid_abi_gen::TrivialAbi::new(
                            Self::generate_ty_name(),
                            Self::generate_internal_ty_name(),
                            name),
                        components,
                    }
                )
            }
        }
        #[cfg(feature = "liquid-abi-gen")]
        impl liquid_abi_gen::traits::GenerateOutputs for #ident {
            fn generate_outputs<B>(builder: &mut B)
            where
                B: liquid_abi_gen::traits::FnOutputBuilder
            {
                let param_abi = <Self as liquid_abi_gen::traits::GenerateParamAbi>::generate_param_abi("".into());
                builder.output(param_abi);
            }
        }
    }
}

fn generate_abi_enum(ident: &Ident, variants: &[Variant]) -> TokenStream2 {
    let variant_abis = variants
        .iter()
        .map(|variant| {
            let ty = variant.ident.to_string();

            if variant.is_unit {
                return quote! {
                    liquid_abi_gen::ParamAbi::Composite(
                        liquid_abi_gen::CompositeAbi {
                            trivial: liquid_abi_gen::TrivialAbi::new(
                                String::from(#ty),
                                String::new(),
                                String::new()),
                            components: Vec::new(),
                        }
                    )
                }
            }

            let field_abis = if variant.unnamed {
                variant.field_tys.iter().map(|field_ty| {
                    quote! {
                        <#field_ty as liquid_abi_gen::traits::GenerateParamAbi>::generate_param_abi(String::new())
                    }
                }).collect::<Vec<_>>()
            } else {
                variant.field_names
                    .iter()
                    .map(|name| name.to_string())
                    .zip(variant.field_tys.iter())
                    .map(|(field_name, field_ty)| {
                        quote! {
                            <#field_ty as liquid_abi_gen::traits::GenerateParamAbi>::generate_param_abi(#field_name.to_owned())
                        }
                }).collect::<Vec<_>>()
            };

            quote! {
                liquid_abi_gen::ParamAbi::Composite(
                    liquid_abi_gen::CompositeAbi {
                        trivial: liquid_abi_gen::TrivialAbi::new(
                            String::from(#ty),
                            String::new(),
                            String::new()),
                        components: {
                            let mut components = Vec::new();
                            #(components.push(#field_abis);)*
                            components
                        },
                    }
                )
            }
        });

    let ident_str = ident.to_string();
    quote! {
        #[cfg(feature = "liquid-abi-gen")]
        impl liquid_abi_gen::traits::GenerateParamAbi for #ident {
            fn generate_ty_name() -> liquid_prelude::string::String {
                String::from("enum")
            }

            fn generate_internal_ty_name() -> liquid_prelude::string::String {
                let mut internal_ty_name = String::from("enum.");
                internal_ty_name.push_str(#ident_str);
                internal_ty_name
            }

            fn generate_param_abi(name: String) -> liquid_abi_gen::ParamAbi {
                let mut components = Vec::new();
                #(components.push(#variant_abis);)*
                liquid_abi_gen::ParamAbi::Composite(
                    liquid_abi_gen::CompositeAbi {
                        trivial: liquid_abi_gen::TrivialAbi::new(
                            Self::generate_ty_name(),
                            Self::generate_internal_ty_name(),
                            name),
                        components,
                    }
                )
            }
        }

        #[cfg(feature = "liquid-abi-gen")]
        impl liquid_abi_gen::traits::GenerateOutputs for #ident {
            fn generate_outputs<B>(builder: &mut B)
            where
                B: liquid_abi_gen::traits::FnOutputBuilder
            {
                let param_abi = <Self as liquid_abi_gen::traits::GenerateParamAbi>::generate_param_abi("".into());
                builder.output(param_abi);
            }
        }
    }
}
