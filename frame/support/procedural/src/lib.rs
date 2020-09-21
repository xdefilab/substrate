// This file is part of Substrate.

// Copyright (C) 2017-2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Proc macro of Support code for the runtime.

#![recursion_limit="512"]

mod storage;
mod construct_runtime;
mod pallet;
mod transactional;

pub(crate) use storage::INHERENT_INSTANCE_NAME;
use proc_macro::TokenStream;

/// Declares strongly-typed wrappers around codec-compatible types in storage.
///
/// ## Example
///
/// ```nocompile
/// decl_storage! {
/// 	trait Store for Module<T: Trait> as Example {
/// 		Foo get(fn foo) config(): u32=12;
/// 		Bar: map hasher(identity) u32 => u32;
/// 		pub Zed build(|config| vec![(0, 0)]): map hasher(identity) u32 => u32;
/// 	}
/// }
/// ```
///
/// Declaration is set with the header `(pub) trait Store for Module<T: Trait> as Example`,
/// with `Store` a (pub) trait generated associating each storage item to the `Module` and
/// `as Example` setting the prefix used for storage items of this module. `Example` must be unique:
/// another module with the same name and the same inner storage item name will conflict.
/// `Example` is called the module prefix.
///
/// note: For instantiable modules the module prefix is prepended with instance
/// prefix. Instance prefix is "" for default instance and "Instance$n" for instance number $n.
/// Thus, instance 3 of module Example has a module prefix of `Instance3Example`
///
/// Basic storage consists of a name and a type; supported types are:
///
/// * Value: `Foo: type`: Implements the
///   [`StorageValue`](../frame_support/storage/trait.StorageValue.html) trait using the
///   [`StorageValue generator`](../frame_support/storage/generator/trait.StorageValue.html).
///
///   The generator is implemented with:
///   * `module_prefix`: module_prefix
///   * `storage_prefix`: storage_name
///
///   Thus the storage value is finally stored at:
///   ```nocompile
///   Twox128(module_prefix) ++ Twox128(storage_prefix)
///   ```
///
/// * Map: `Foo: map hasher($hash) type => type`: Implements the
///   [`StorageMap`](../frame_support/storage/trait.StorageMap.html) trait using the
///   [`StorageMap generator`](../frame_support/storage/generator/trait.StorageMap.html).
///   And [`StoragePrefixedMap`](../frame_support/storage/trait.StoragePrefixedMap.html).
///
///   `$hash` representing a choice of hashing algorithms available in the
///   [`Hashable`](../frame_support/trait.Hashable.html) trait. You will generally want to use one
///   of three hashers:
///   * `blake2_128_concat`: The default, safe choice. Use if you are unsure or don't care. It is
///     secure against user-tainted keys, fairly fast and memory-efficient and supports
///     iteration over its keys and values. This must be used if the keys of your map can be
///     selected *en masse* by untrusted users.
///   * `twox_64_concat`: This is an insecure hasher and can only be used safely if you know that
///     the preimages cannot be chosen at will by untrusted users. It is memory-efficient, extremely
///     performant and supports iteration over its keys and values. You can safely use this is the
///     key is:
///     - A (slowly) incrementing index.
///     - Known to be the result of a cryptographic hash (though `identity` is a better choice here).
///     - Known to be the public key of a cryptographic key pair in existence.
///   * `identity`: This is not a hasher at all, and just uses the key material directly. Since it
///     does no hashing or appending, it's the fastest possible hasher, however, it's also the least
///     secure. It can be used only if you know that the key will be cryptographically/securely
///     randomly distributed over the binary encoding space. In most cases this will not be true.
///     One case where it is true, however, if where the key is itself the result of a cryptographic
///     hash of some existent data.
///
///   Other hashers will tend to be "opaque" and not support iteration over the keys in the
///   map. It is not recommended to use these.
///
///   The generator is implemented with:
///   * `module_prefix`: $module_prefix
///   * `storage_prefix`: storage_name
///   * `Hasher`: $hash
///
///   Thus the keys are stored at:
///   ```nocompile
///   twox128(module_prefix) ++ twox128(storage_prefix) ++ hasher(encode(key))
///   ```
///
/// * Double map: `Foo: double_map hasher($hash1) u32, hasher($hash2) u32 => u32`: Implements the
///   [`StorageDoubleMap`](../frame_support/storage/trait.StorageDoubleMap.html) trait using the
///   [`StorageDoubleMap generator`](../frame_support/storage/generator/trait.StorageDoubleMap.html).
///   And [`StoragePrefixedMap`](../frame_support/storage/trait.StoragePrefixedMap.html).
///
///   `$hash1` and `$hash2` representing choices of hashing algorithms available in the
///   [`Hashable`](../frame_support/trait.Hashable.html) trait. They must be chosen with care, see
///   generator documentation.
///
///   The generator is implemented with:
///   * `module_prefix`: $module_prefix
///   * `storage_prefix`: storage_name
///   * `Hasher1`: $hash1
///   * `Hasher2`: $hash2
///
///   Thus keys are stored at:
///   ```nocompile
///   Twox128(module_prefix) ++ Twox128(storage_prefix) ++ Hasher1(encode(key1)) ++ Hasher2(encode(key2))
///   ```
///
/// Supported hashers (ordered from least to best security):
///
/// * `identity` - Just the unrefined key material. Use only when it is known to be a secure hash
///   already. The most efficient and iterable over keys.
/// * `twox_64_concat` - TwoX with 64bit + key concatenated. Use only when an untrusted source
///   cannot select and insert key values. Very efficient and iterable over keys.
/// * `blake2_128_concat` - Blake2 with 128bit + key concatenated. Slower but safe to use in all
///   circumstances. Iterable over keys.
///
/// Deprecated hashers, which do not support iteration over keys include:
/// * `twox_128` - TwoX with 128bit.
/// * `twox_256` - TwoX with with 256bit.
/// * `blake2_128` - Blake2 with 128bit.
/// * `blake2_256` - Blake2 with 256bit.
///
/// Basic storage can be extended as such:
///
/// `#vis #name get(fn #getter) config(#field_name) build(#closure): #type = #default;`
///
/// * `#vis`: Set the visibility of the structure. `pub` or nothing.
/// * `#name`: Name of the storage item, used as a prefix in storage.
/// * \[optional\] `get(fn #getter)`: Implements the function #getter to `Module`.
/// * \[optional\] `config(#field_name)`: `field_name` is optional if get is set.
/// Will include the item in `GenesisConfig`.
/// * \[optional\] `build(#closure)`: Closure called with storage overlays.
/// * `#type`: Storage type.
/// * \[optional\] `#default`: Value returned when none.
///
/// Storage items are accessible in multiple ways:
///
/// * The structure: `Foo` or `Foo::<T>` depending if the value type is generic or not.
/// * The `Store` trait structure: `<Module<T> as Store>::Foo`
/// * The getter on the module that calls get on the structure: `Module::<T>::foo()`
///
/// ## GenesisConfig
///
/// An optional `GenesisConfig` struct for storage initialization can be defined, either
/// when at least one storage field requires default initialization
/// (both `get` and `config` or `build`), or specifically as in:
///
/// ```nocompile
/// decl_storage! {
/// 	trait Store for Module<T: Trait> as Example {
///
/// 		// Your storage items
/// 	}
///		add_extra_genesis {
///			config(genesis_field): GenesisFieldType;
///			config(genesis_field2): GenesisFieldType;
///			...
///			build(|_: &Self| {
///				// Modification of storage
///			})
///		}
/// }
/// ```
///
/// This struct can be exposed as `ExampleConfig` by the `construct_runtime!` macro like follows:
///
/// ```nocompile
/// construct_runtime!(
/// 	pub enum Runtime with ... {
///         ...,
///         Example: example::{Module, Storage, ..., Config<T>},
///         ...,
///	}
/// );
/// ```
///
/// ### Module with Instances
///
/// The `decl_storage!` macro supports building modules with instances with the following syntax
/// (`DefaultInstance` type is optional):
///
/// ```nocompile
/// trait Store for Module<T: Trait<I>, I: Instance=DefaultInstance> as Example {}
/// ```
///
/// Accessing the structure no requires the instance as generic parameter:
/// * `Foo::<I>` if the value type is not generic
/// * `Foo::<T, I>` if the value type is generic
///
/// ## Where clause
///
/// This macro supports a where clause which will be replicated to all generated types.
///
/// ```nocompile
/// trait Store for Module<T: Trait> as Example where T::AccountId: std::fmt::Display {}
/// ```
///
/// ## Limitations
///
/// # Instancing and generic `GenesisConfig`
///
/// If your module supports instancing and you see an error like `parameter `I` is never used` for
/// your `decl_storage!`, you are hitting a limitation of the current implementation. You probably
/// try to use an associated type of a non-instantiable trait. To solve this, add the following to
/// your macro call:
///
/// ```nocompile
/// add_extra_genesis {
/// 	config(phantom): std::marker::PhantomData<I>,
/// }
/// ...
///
/// This adds a field to your `GenesisConfig` with the name `phantom` that you can initialize with
/// `Default::default()`.
///
#[proc_macro]
pub fn decl_storage(input: TokenStream) -> TokenStream {
	storage::decl_storage_impl(input)
}

/// Construct a runtime, with the given name and the given modules.
///
/// The parameters here are specific types for `Block`, `NodeBlock`, and `UncheckedExtrinsic`
/// and the modules that are used by the runtime.
/// `Block` is the block type that is used in the runtime and `NodeBlock` is the block type
/// that is used in the node. For instance they can differ in the extrinsics type.
///
/// # Example:
///
/// ```nocompile
/// construct_runtime!(
///     pub enum Runtime where
///         Block = Block,
///         NodeBlock = runtime::Block,
///         UncheckedExtrinsic = UncheckedExtrinsic
///     {
///         System: system::{Module, Call, Event<T>, Config<T>},
///         Test: test::{Module, Call},
///         Test2: test_with_long_module::{Module},
///
///         // Module with instances
///         Test3_Instance1: test3::<Instance1>::{Module, Call, Storage, Event<T, I>, Config<T, I>, Origin<T, I>},
///         Test3_DefaultInstance: test3::{Module, Call, Storage, Event<T>, Config<T>, Origin<T>},
///     }
/// )
/// ```
///
/// The identifier `System` is the name of the pallet and the lower case identifier `system` is the
/// name of the Rust module/crate for this Substrate module. The identifiers between the braces are
/// the module parts provided by the pallet. It is important to list these parts here to export
/// them correctly in the metadata or to make the pallet usable in the runtime.
///
/// We provide support for the following module parts in a pallet:
///
/// - `Module`
/// - `Call`
/// - `Storage`
/// - `Event` or `Event<T>` (if the event is generic)
/// - `Origin` or `Origin<T>` (if the origin is generic)
/// - `Config` or `Config<T>` (if the config is generic)
/// - `Inherent` - If the module provides/can check inherents.
/// - `ValidateUnsigned` - If the module validates unsigned extrinsics.
///
/// # Note
///
/// The population of the genesis storage depends on the order of modules. So, if one of your
/// modules depends on another module, the module that is depended upon needs to come before
/// the module depending on it.
#[proc_macro]
pub fn construct_runtime(input: TokenStream) -> TokenStream {
	construct_runtime::construct_runtime(input)
}

/// Macro to define a pallet. Docs are at `frame_support::pallet`.
#[proc_macro_attribute]
pub fn pallet(attr: TokenStream, item: TokenStream) -> TokenStream {
	pallet::pallet(attr, item)
}

/// Derive Clone but do not bound any generic.
#[proc_macro_derive(CloneNoBound)]
pub fn derive_clone_no_bound(input: TokenStream) -> TokenStream {
	use syn::spanned::Spanned;

	let input: syn::DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};

	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	let impl_ = match input.data {
		syn::Data::Struct(struct_) => match struct_.fields {
			syn::Fields::Named(named) => {
				let fields = named.named.iter()
					.map(|i| i.ident.as_ref().expect("named fields have ident"))
					.map(|i| quote::quote_spanned!(i.span() => #i: self.#i.clone() ));

				quote::quote!( Self { #( #fields, )* } )
			},
			syn::Fields::Unnamed(unnamed) => {
				let fields = unnamed.unnamed.iter().enumerate()
					.map(|(i, _)| syn::Index::from(i))
					.map(|i| quote::quote_spanned!(i.span() => self.#i.clone() ));

				quote::quote!( Self ( #( #fields, )* ) )
			},
			syn::Fields::Unit => {
				quote::quote!( Self )
			}
		},
		syn::Data::Enum(enum_) => {
			let variants = enum_.variants.iter()
				.map(|variant| {
					let ident = &variant.ident;
					match &variant.fields {
						syn::Fields::Named(named) => {
							let captured = named.named.iter()
								.map(|i| i.ident.as_ref().expect("named fields have ident"));
							let cloned = captured.clone()
								.map(|i| quote::quote_spanned!(i.span() =>
									#i: core::clone::Clone::clone(#i)
								));
							quote::quote!(
								Self::#ident { #( ref #captured, )* } => Self::#ident { #( #cloned, )*}
							)
						},
						syn::Fields::Unnamed(unnamed) => {
							let captured = unnamed.unnamed.iter().enumerate()
								.map(|(i, f)| syn::Ident::new(&format!("_{}", i), f.span()));
							let cloned = captured.clone()
								.map(|i| quote::quote_spanned!(i.span() =>
									core::clone::Clone::clone(#i)
								));
							quote::quote!(
								Self::#ident ( #( ref #captured, )* ) => Self::#ident ( #( #cloned, )*)
							)
						},
						syn::Fields::Unit => quote::quote!( Self::#ident => Self::#ident ),
					}
				});

			quote::quote!( match self {
				#( #variants, )*
			})
		},
		syn::Data::Union(_) => {
			let msg ="Union type not supported by `derive(CloneNoBound)`";
			return syn::Error::new(input.span(), msg).to_compile_error().into()
		},
	};

	quote::quote!(
		const _: () = {
			impl #impl_generics core::clone::Clone for #name #ty_generics #where_clause {
				fn clone(&self) -> Self {
					#impl_
				}
			}
		};
	).into()
}

/// Derive Debug by returning `"<stripped>"` (also do not bound any generic).
#[proc_macro_derive(DebugStripped)]
pub fn derive_debug_stripped(input: TokenStream) -> TokenStream {
	let input: syn::DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};

	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	quote::quote!(
		const _: () = {
			impl #impl_generics core::fmt::Debug for #name #ty_generics #where_clause {
				fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
					fmt.write_str("<stripped>")
				}
			}
		};
	).into()
}

/// Derive Debug but do not bound any generics.
#[proc_macro_derive(DebugNoBound)]
pub fn derive_debug_no_bound(input: TokenStream) -> TokenStream {
	use syn::spanned::Spanned;

	let input: syn::DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};

	let input_ident = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	let impl_ = match input.data {
		syn::Data::Struct(struct_) => match struct_.fields {
			syn::Fields::Named(named) => {
				let fields = named.named.iter()
					.map(|i| i.ident.as_ref().expect("named fields have ident"))
					.map(|i| quote::quote_spanned!(i.span() => .field(stringify!(#i), &self.#i) ));

				quote::quote!( fmt.debug_struct(stringify!(#input_ident))
					#( #fields )*
					.finish()
				)
			},
			syn::Fields::Unnamed(unnamed) => {
				let fields = unnamed.unnamed.iter().enumerate()
					.map(|(i, _)| syn::Index::from(i))
					.map(|i| quote::quote_spanned!(i.span() => .field(&self.#i) ));

				quote::quote!( fmt.debug_tuple(stringify!(#input_ident))
					#( #fields )*
					.finish()
				)
			},
			syn::Fields::Unit => quote::quote!( fmt.write_str(stringify!(#input_ident)) ),
		},
		syn::Data::Enum(enum_) => {
			let variants = enum_.variants.iter()
				.map(|variant| {
					let ident = &variant.ident;
					let full_variant_str = format!("{}::{}", input_ident, ident);
					match &variant.fields {
						syn::Fields::Named(named) => {
							let captured = named.named.iter()
								.map(|i| i.ident.as_ref().expect("named fields have ident"));
							let debuged = captured.clone()
								.map(|i| quote::quote_spanned!(i.span() =>
									.field(stringify!(#i), &#i)
								));
							quote::quote!(
								Self::#ident { #( ref #captured, )* } => {
									fmt.debug_struct(#full_variant_str)
										#( #debuged )*
										.finish()
								}
							)
						},
						syn::Fields::Unnamed(unnamed) => {
							let captured = unnamed.unnamed.iter().enumerate()
								.map(|(i, f)| syn::Ident::new(&format!("_{}", i), f.span()));
							let debuged = captured.clone()
								.map(|i| quote::quote_spanned!(i.span() => .field(&#i) ));
							quote::quote!(
								Self::#ident ( #( ref #captured, )* ) => {
									fmt.debug_tuple(#full_variant_str)
										#( #debuged )*
										.finish()
								}
							)
						},
						syn::Fields::Unit => quote::quote!(
							Self::#ident => fmt.write_str(#full_variant_str)
						),
					}
				});

			quote::quote!( match *self {
				#( #variants, )*
			})
		},
		syn::Data::Union(_) => {
			let msg ="Union type not supported by `derive(DebugNoBound)`";
			return syn::Error::new(input.span(), msg).to_compile_error().into()
		},
	};

	quote::quote!(
		const _: () = {
			impl #impl_generics core::fmt::Debug for #input_ident #ty_generics #where_clause {
				fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
					#impl_
				}
			}
		};
	).into()
}


/// Derive PartialEq but do not bound any generic.
#[proc_macro_derive(PartialEqNoBound)]
pub fn derive_partial_eq_no_bound(input: TokenStream) -> TokenStream {
	use syn::spanned::Spanned;

	let input: syn::DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};

	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	let impl_ = match input.data {
		syn::Data::Struct(struct_) => match struct_.fields {
			syn::Fields::Named(named) => {
				let fields = named.named.iter()
					.map(|i| i.ident.as_ref().expect("named fields have ident"))
					.map(|i| quote::quote_spanned!(i.span() => self.#i == other.#i ));

				quote::quote!( true #( && #fields )* )
			},
			syn::Fields::Unnamed(unnamed) => {
				let fields = unnamed.unnamed.iter().enumerate()
					.map(|(i, _)| syn::Index::from(i))
					.map(|i| quote::quote_spanned!(i.span() => self.#i == other.#i ));

				quote::quote!( true #( && #fields )* )
			},
			syn::Fields::Unit => {
				quote::quote!( true )
			}
		},
		syn::Data::Enum(enum_) => {
			let variants = enum_.variants.iter()
				.map(|variant| {
					let ident = &variant.ident;
					match &variant.fields {
						syn::Fields::Named(named) => {
							let names = named.named.iter()
								.map(|i| i.ident.as_ref().expect("named fields have ident"));
							let names_bis = names.clone()
								.map(|i| syn::Ident::new(&format!("{}_bis", i), i.span()));

							let capture = names.clone();
							let capture_bis = names.clone().zip(names_bis.clone())
								.map(|(i, i_bis)| quote::quote!(#i: #i_bis));
							let eq = names.zip(names_bis)
								.map(|(i, i_bis)| quote::quote_spanned!(i.span() => #i == #i_bis));
							quote::quote!(
								(
									Self::#ident { #( #capture, )* },
									Self::#ident { #( #capture_bis, )* },
								) => true #( && #eq )*
							)
						},
						syn::Fields::Unnamed(unnamed) => {
							let names = unnamed.unnamed.iter().enumerate()
								.map(|(i, f)| syn::Ident::new(&format!("_{}", i), f.span()));
							let names_bis = unnamed.unnamed.iter().enumerate()
								.map(|(i, f)| syn::Ident::new(&format!("_{}_bis", i), f.span()));
							let eq = names.clone().zip(names_bis.clone())
								.map(|(i, i_bis)| quote::quote_spanned!(i.span() => #i == #i_bis));
							quote::quote!(
								(
									Self::#ident ( #( #names, )* ),
									Self::#ident ( #( #names_bis, )* ),
								) => true #( && #eq )*
							)
						},
						syn::Fields::Unit => quote::quote!( (Self::#ident, Self::#ident) => true ),
					}
				});

			quote::quote!( match (self, other) {
				#( #variants, )*
				_ => false,
			})
		},
		syn::Data::Union(_) => {
			let msg ="Union type not supported by `derive(PartialEqNoBound)`";
			return syn::Error::new(input.span(), msg).to_compile_error().into()
		},
	};

	quote::quote!(
		const _: () = {
			impl #impl_generics core::cmp::PartialEq for #name #ty_generics #where_clause {
				fn eq(&self, other: &Self) -> bool {
					#impl_
				}
			}
		};
	).into()
}

/// derive Eq but do no bound any generic.
#[proc_macro_derive(EqNoBound)]
pub fn derive_eq_no_bound(input: TokenStream) -> TokenStream {
	let input: syn::DeriveInput = match syn::parse(input) {
		Ok(input) => input,
		Err(e) => return e.to_compile_error().into(),
	};

	let name = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	quote::quote!(
		const _: () = {
			impl #impl_generics core::cmp::Eq for #name #ty_generics #where_clause {}
		};
	).into()
}

/// Execute the annotated function in a new storage transaction.
///
/// The return type of the annotated function must be `Result`. All changes to storage performed
/// by the annotated function are discarded if it returns `Err`, or committed if `Ok`.
///
/// # Example
///
/// ```nocompile
/// #[transactional]
/// fn value_commits(v: u32) -> result::Result<u32, &'static str> {
/// 	Value::set(v);
/// 	Ok(v)
/// }
///
/// #[transactional]
/// fn value_rollbacks(v: u32) -> result::Result<u32, &'static str> {
/// 	Value::set(v);
/// 	Err("nah")
/// }
/// ```
#[proc_macro_attribute]
pub fn transactional(attr: TokenStream, input: TokenStream) -> TokenStream {
	transactional::transactional(attr, input).unwrap_or_else(|e| e.to_compile_error().into())
}
