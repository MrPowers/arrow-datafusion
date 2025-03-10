// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Data source traits

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use datafusion_common::{DataFusionError, Statistics};
use datafusion_expr::{CreateExternalTable, LogicalPlan};
pub use datafusion_expr::{TableProviderFilterPushDown, TableType};

use crate::arrow::datatypes::SchemaRef;
use crate::error::Result;
use crate::execution::context::SessionState;
use crate::logical_expr::Expr;
use crate::physical_plan::ExecutionPlan;

/// Source table
#[async_trait]
pub trait TableProvider: Sync + Send {
    /// Returns the table provider as [`Any`](std::any::Any) so that it can be
    /// downcast to a specific implementation.
    fn as_any(&self) -> &dyn Any;

    /// Get a reference to the schema for this table
    fn schema(&self) -> SchemaRef;

    /// Get the type of this table for metadata/catalog purposes.
    fn table_type(&self) -> TableType;

    /// Get the create statement used to create this table, if available.
    fn get_table_definition(&self) -> Option<&str> {
        None
    }

    /// Get the Logical Plan of this table, if available.
    fn get_logical_plan(&self) -> Option<&LogicalPlan> {
        None
    }

    /// Create an ExecutionPlan that will scan the table.
    /// The table provider will be usually responsible of grouping
    /// the source data into partitions that can be efficiently
    /// parallelized or distributed.
    async fn scan(
        &self,
        state: &SessionState,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        // limit can be used to reduce the amount scanned
        // from the datasource as a performance optimization.
        // If set, it contains the amount of rows needed by the `LogicalPlan`,
        // The datasource should return *at least* this number of rows if available.
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>>;

    /// Tests whether the table provider can make use of a filter expression
    /// to optimise data retrieval.
    #[deprecated(since = "20.0.0", note = "use supports_filters_pushdown instead")]
    fn supports_filter_pushdown(
        &self,
        _filter: &Expr,
    ) -> Result<TableProviderFilterPushDown> {
        Ok(TableProviderFilterPushDown::Unsupported)
    }

    /// Tests whether the table provider can make use of any or all filter expressions
    /// to optimise data retrieval.
    #[allow(deprecated)]
    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> Result<Vec<TableProviderFilterPushDown>> {
        filters
            .iter()
            .map(|f| self.supports_filter_pushdown(f))
            .collect()
    }

    /// Get statistics for this table, if available
    fn statistics(&self) -> Option<Statistics> {
        None
    }

    /// Return an [`ExecutionPlan`] to insert data into this table, if
    /// supported.
    ///
    /// The returned plan should return a single row in a UInt64
    /// column called "count" such as the following
    ///
    /// ```text
    /// +-------+,
    /// | count |,
    /// +-------+,
    /// | 6     |,
    /// +-------+,
    /// ```
    ///
    /// # See Also
    ///
    /// See [`InsertExec`] for the common pattern of inserting a
    /// single stream of `RecordBatch`es.
    ///
    /// [`InsertExec`]: crate::physical_plan::insert::InsertExec
    async fn insert_into(
        &self,
        _state: &SessionState,
        _input: Arc<dyn ExecutionPlan>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        let msg = "Insertion not implemented for this table".to_owned();
        Err(DataFusionError::NotImplemented(msg))
    }
}

/// A factory which creates [`TableProvider`]s at runtime given a URL.
///
/// For example, this can be used to create a table "on the fly"
/// from a directory of files only when that name is referenced.
#[async_trait]
pub trait TableProviderFactory: Sync + Send {
    /// Create a TableProvider with the given url
    async fn create(
        &self,
        state: &SessionState,
        cmd: &CreateExternalTable,
    ) -> Result<Arc<dyn TableProvider>>;
}
