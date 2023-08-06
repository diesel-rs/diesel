use syn::parse_quote;

use super::{case::Case, DeriveSettings, InferrerSettings};

#[derive(Default)]
pub struct DeriveSettingsBuilder {
    inner: DeriveSettings,
}

impl DeriveSettingsBuilder {
    pub fn default_dsl_path(mut self, path: syn::Path) -> Self {
        self.inner.default_dsl_path = path;
        self
    }

    pub fn default_generate_type_alias(mut self, generate_type_alias: bool) -> Self {
        self.inner.default_generate_type_alias = generate_type_alias;
        self
    }

    pub fn default_method_type_case(mut self, case: Case) -> Self {
        self.inner.default_method_type_case = case;
        self
    }

    pub fn default_function_type_case(mut self, case: Case) -> Self {
        self.inner.default_function_type_case = case;
        self
    }

    pub fn build(self) -> DeriveSettings {
        self.inner
    }
}

impl DeriveSettings {
    pub fn builder() -> DeriveSettingsBuilder {
        DeriveSettingsBuilder::default()
    }
}

impl Default for DeriveSettings {
    fn default() -> Self {
        Self {
            default_method_type_case: Case::UpperCamel,
            default_function_type_case: Case::DoNotChange,
            default_dsl_path: parse_quote!(dsl),
            default_generate_type_alias: true,
        }
    }
}

#[derive(Default)]
pub struct InferrerSettingsBuilder {
    inner: InferrerSettings,
}

impl InferrerSettingsBuilder {
    pub fn dsl_path(mut self, path: syn::Path) -> Self {
        self.inner.dsl_path = path;
        self
    }

    pub fn method_types_case(mut self, case: Case) -> Self {
        self.inner.method_types_case = case;
        self
    }

    pub fn function_types_case(mut self, case: Case) -> Self {
        self.inner.function_types_case = case;
        self
    }

    pub fn build(self) -> InferrerSettings {
        self.inner
    }
}

impl InferrerSettings {
    pub fn builder() -> InferrerSettingsBuilder {
        InferrerSettingsBuilder::default()
    }
}

impl Default for InferrerSettings {
    fn default() -> Self {
        Self {
            method_types_case: Case::UpperCamel,
            function_types_case: Case::DoNotChange,
            dsl_path: parse_quote!(dsl),
        }
    }
}
