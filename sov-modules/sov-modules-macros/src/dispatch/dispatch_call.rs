use super::common::parse_generic_params;
use super::common::StructDef;
use super::common::StructFieldExtractor;
use super::common::CALL;
use syn::DeriveInput;

impl<'a> StructDef<'a> {
    fn create_call_enum_legs(&self) -> Vec<proc_macro2::TokenStream> {
        self.fields
            .iter()
            .map(|field| {
                let name = &field.ident;
                let ty = &field.ty;

                quote::quote!(
                    #name(<#ty as sov_modules_api::Module>::CallMessage),
                )
            })
            .collect()
    }

    fn create_call_dispatch(&self) -> proc_macro2::TokenStream {
        let enum_ident = self.enum_ident(CALL);
        let type_generics = &self.type_generics;

        let match_legs = self.fields.iter().map(|field| {
            let name = &field.ident;

            quote::quote!(
                #enum_ident::#name(message)=>{
                    sov_modules_api::Module::call(&mut self.#name, message, context, working_set)
                },
            )
        });

        let match_legs_address = self.fields.iter().map(|field| {
            let name = &field.ident;
            let ty = &field.ty;

            quote::quote!(
                #enum_ident::#name(message)=>{
                    <#ty as sov_modules_api::ModuleInfo>::address()

                },
            )
        });

        let ident = &self.ident;
        let impl_generics = &self.impl_generics;
        let where_clause = self.where_clause;
        let generic_param = self.generic_param;
        let ty_generics = &self.type_generics;
        let call_enum = self.enum_ident(CALL);

        quote::quote! {
            impl #impl_generics sov_modules_api::DispatchCall for #ident #type_generics #where_clause {
                type Context = #generic_param;
                type Decodable = #call_enum #ty_generics;


                fn decode_call(serialized_message: &[u8]) -> core::result::Result<Self::Decodable, std::io::Error> {
                    let mut data = std::io::Cursor::new(serialized_message);
                    <#call_enum #ty_generics as sovereign_sdk::serial::Decode>::decode(&mut data)
                }

                fn dispatch_call(
                    &mut self,
                    decodable: Self::Decodable,
                    working_set: &mut sov_state::WorkingSet<<Self::Context as sov_modules_api::Spec>::Storage>,
                    context: &Self::Context,
                ) -> core::result::Result<sov_modules_api::CallResponse, sov_modules_api::Error> {

                    match decodable {
                        #(#match_legs)*
                    }

                }

                fn module_address(&self, decodable: &Self::Decodable) -> sov_modules_api::Address {
                    match decodable {
                        #(#match_legs_address)*
                    }
                }
            }
        }
    }
}

pub(crate) struct DispatchCallMacro {
    field_extractor: StructFieldExtractor,
}

impl DispatchCallMacro {
    pub(crate) fn new(name: &'static str) -> Self {
        Self {
            field_extractor: StructFieldExtractor::new(name),
        }
    }

    pub(crate) fn derive_dispatch_call(
        &self,
        input: DeriveInput,
    ) -> Result<proc_macro::TokenStream, syn::Error> {
        let DeriveInput {
            data,
            ident,
            generics,
            ..
        } = input;

        let generic_param = parse_generic_params(&generics)?;

        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
        let fields = self.field_extractor.get_fields_from_struct(&data)?;

        let struct_def = StructDef::new(
            ident,
            fields,
            impl_generics,
            type_generics,
            &generic_param,
            where_clause,
        );

        let call_enum_legs = struct_def.create_call_enum_legs();
        let call_enum = struct_def.create_enum(&call_enum_legs, CALL);
        let create_dispatch_impl = struct_def.create_call_dispatch();

        Ok(quote::quote! {

            #call_enum

            #create_dispatch_impl
        }
        .into())
    }
}
