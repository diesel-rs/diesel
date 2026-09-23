use super::{
    ColumnDefinition, QueryRelationData, SupportedQueryRelationStructures, TableName,
    load_table_data_for_query_resolution, load_table_names, load_view_data,
    order_query_relation_columns_for_output, resolve_unqualified_relation_schema,
};
use crate::config::PrintSchema;
use crate::database::InferConnection;
use diesel_infer_query::{IsNull, SchemaField, SchemaResolver};
use std::collections::HashMap;

pub struct SchemaResolverImpl<'a, 'b> {
    pub(super) connection: &'a mut InferConnection,
    print_schema_relations: Vec<(SupportedQueryRelationStructures, TableName)>,
    cached_results: HashMap<TableName, QueryRelationData>,
    pub(super) config: &'b PrintSchema,
    unfiltered_table_names: HashMap<TableName, SupportedQueryRelationStructures>,
    recursive_resolve_chain: Vec<TableName>,
    resolved_unqualified_relations: HashMap<String, TableName>,
}

impl<'a, 'b> SchemaResolverImpl<'a, 'b> {
    pub(crate) fn new(
        connection: &'a mut InferConnection,
        relations: Vec<(SupportedQueryRelationStructures, TableName)>,
        config: &'b PrintSchema,
        unfiltered_table_names: Vec<(SupportedQueryRelationStructures, TableName)>,
    ) -> Self {
        let unfiltered_table_names = unfiltered_table_names
            .into_iter()
            .map(|(t, rel)| (rel, t))
            .collect();
        Self {
            connection,
            print_schema_relations: relations,
            cached_results: HashMap::new(),
            config,
            unfiltered_table_names,
            recursive_resolve_chain: Vec::new(),
            resolved_unqualified_relations: HashMap::new(),
        }
    }

    pub(crate) fn resolve_query_relations(
        mut self,
    ) -> Result<Vec<QueryRelationData>, crate::errors::Error> {
        let requested_relations = self.print_schema_relations.clone();
        for (kind, t) in requested_relations {
            debug_assert!(self.recursive_resolve_chain.is_empty());
            self.load_query_relation_data(Some(kind), t)?;
        }
        debug_assert!(self.recursive_resolve_chain.is_empty());

        // extract all data required for the actual print schema operation
        //
        // Our `cached_results` list could contain many more table entries at this
        // point as loading views could trigger loading additional data
        let mut output = Vec::with_capacity(self.print_schema_relations.len());
        for (_, relation_name) in self.print_schema_relations {
            let mut relation = self
                .cached_results
                .remove(&relation_name)
                .expect("This relation was loaded before");
            order_query_relation_columns_for_output(self.connection, self.config, &mut relation)?;
            output.push(relation);
        }
        Ok(output)
    }

    fn load_query_relation_data(
        &mut self,
        kind: Option<SupportedQueryRelationStructures>,
        t: TableName,
    ) -> Result<&QueryRelationData, crate::errors::Error> {
        // use this construct to make borrowck understand
        // that we don't borrow things mutably at the same time
        if !self.cached_results.contains_key(&t) {
            if self.recursive_resolve_chain.contains(&t) {
                tracing::error!(chain = ?self.recursive_resolve_chain, "Cyclic view definition");
                return Err(crate::errors::Error::CyclicViewDefinition(t));
            }

            self.recursive_resolve_chain.push(t.clone());
            let data = self.load_uncached_query_relation_data(kind, &t);
            let popped = self.recursive_resolve_chain.pop();
            debug_assert_eq!(popped.as_ref(), Some(&t));
            self.cached_results.insert(t.clone(), data?);
        }
        Ok(self
            .cached_results
            .get(&t)
            .expect("We literally inserted that above"))
    }

    fn load_uncached_query_relation_data(
        &mut self,
        kind: Option<SupportedQueryRelationStructures>,
        t: &TableName,
    ) -> Result<QueryRelationData, crate::errors::Error> {
        let kind = match kind.or_else(|| self.unfiltered_table_names.get(t).copied()) {
            Some(kind) => kind,
            None => {
                let tables = load_table_names(self.connection, t.schema.as_deref())?;
                self.unfiltered_table_names
                    .extend(tables.into_iter().map(|(tpe, rel)| (rel, tpe)));
                self.unfiltered_table_names.get(t).copied().ok_or_else(|| {
                    tracing::info!(chain = ?self.recursive_resolve_chain, "Resolve chain");
                    crate::errors::Error::CouldNotResolveView(t.clone())
                })?
            }
        };
        match kind {
            SupportedQueryRelationStructures::Table => Ok(QueryRelationData::Table(
                load_table_data_for_query_resolution(
                    self.connection,
                    t.clone(),
                    self.config,
                    kind,
                )?,
            )),
            SupportedQueryRelationStructures::View => {
                Ok(QueryRelationData::View(load_view_data(self, t.clone())?))
            }
        }
    }
}

impl<'a> SchemaResolver for SchemaResolverImpl<'a, '_> {
    fn resolve_field(
        &mut self,
        schema: Option<&str>,
        query_relation: Option<&str>,
        field_name: &str,
    ) -> Result<
        &dyn diesel_infer_query::SchemaField,
        Box<dyn std::error::Error + Send + Sync + 'static>,
    > {
        let Some(query_relation) = query_relation else {
            return Err("Unnamed query source cannot be resolved".into());
        };
        let (table_name, relation) = self.load_relation_data(schema, query_relation)?;
        Ok(relation
            .columns()
            .iter()
            .find_map(|c| (c.sql_name == field_name).then_some(c as &dyn SchemaField))
            .ok_or_else(|| {
                tracing::info!(table = ?table_name, field = %field_name, "Field not found");
                crate::errors::Error::FieldNotFoundForView(table_name, field_name.to_owned())
            })?)
    }

    fn list_fields<'s>(
        &'s mut self,
        relation_schema: Option<&str>,
        query_relation: Option<&str>,
    ) -> Result<Vec<&'s dyn SchemaField>, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let Some(query_relation) = query_relation else {
            return Err("Unnamed query source cannot be resolved".into());
        };
        let (_table_name, relation) = self.load_relation_data(relation_schema, query_relation)?;
        let ret = relation
            .columns()
            .iter()
            .map(|c| c as &dyn SchemaField)
            .collect();

        Ok(ret)
    }
}

impl<'a, 'b> SchemaResolverImpl<'a, 'b> {
    fn load_relation_data(
        &mut self,
        schema: Option<&str>,
        query_relation: &str,
    ) -> Result<(TableName, &QueryRelationData), crate::errors::Error> {
        let table_name = match schema {
            Some(schema) => TableName::new(query_relation, schema),
            None => {
                if let Some(table_name) = self.resolved_unqualified_relations.get(query_relation) {
                    table_name.clone()
                } else {
                    let schema =
                        resolve_unqualified_relation_schema(self.connection, query_relation)?;
                    let table_name = match schema {
                        Some(schema) => TableName::new(query_relation, schema),
                        None => TableName::from_name(query_relation),
                    };
                    self.resolved_unqualified_relations
                        .insert(query_relation.to_owned(), table_name.clone());
                    table_name
                }
            }
        };
        let relation = self.load_query_relation_data(None, table_name.clone())?;
        Ok((table_name, relation))
    }
}

impl SchemaField for ColumnDefinition {
    fn is_nullable(&self) -> IsNull {
        if self.ty.is_nullable {
            IsNull::IsNullable
        } else {
            IsNull::NotNullable
        }
    }

    fn name(&self) -> Option<&str> {
        Some(&self.sql_name)
    }
}
