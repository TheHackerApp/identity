use async_graphql::{
    extensions::{
        Extension, ExtensionContext, ExtensionFactory, NextExecute, NextParseQuery, NextResolve,
        NextValidation, ResolveInfo,
    },
    parser::types::{ExecutableDocument, OperationType, Selection},
    Response, ServerError, ServerResult, ValidationResult, Value, Variables,
};
use std::{sync::Arc, time::Instant};
use tracing::{debug, error, info, span, Instrument, Level};

/// Adds tracing to the GraphQL flow. Adapted from the
/// [`async_graphql::extensions::Tracing`](https://docs.rs/async-graphql/latest/async_graphql/extensions/struct.Tracing.html)
/// and [`async_graphql::extensions::Logger`](https://docs.rs/async-graphql/latest/async_graphql/extensions/struct.Logger.html)
/// extensions.
pub(crate) struct Logging;

impl ExtensionFactory for Logging {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(LoggingExtension)
    }
}

/// The extension implementation
struct LoggingExtension;

#[async_trait::async_trait]
impl Extension for LoggingExtension {
    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let span = span!(Level::INFO, "parse");
        let now = Instant::now();

        async move {
            debug!("started parsing request");
            let result = next.run(ctx, query, variables).await;
            debug!(
                latency = format_args!("{} ms", now.elapsed().as_millis()),
                "finished parsing request"
            );

            let document = result?;

            let is_schema = document
                .operations
                .iter()
                .filter(|(_, operation)| operation.node.ty == OperationType::Query)
                .any(|(_, operation)| operation.node.selection_set.node.items.iter().any(|selection| matches!(&selection.node, Selection::Field(field) if field.node.name.node == "__schema")));
            if !is_schema {
                info!(document = ctx.stringify_execute_doc(&document, variables));
            }

            Ok(document)
        }
        .instrument(span)
        .await
    }

    async fn validation(
        &self,
        ctx: &ExtensionContext<'_>,
        next: NextValidation<'_>,
    ) -> async_graphql::Result<ValidationResult, Vec<ServerError>> {
        let span = span!(Level::INFO, "validation");
        next.run(ctx).instrument(span).await
    }

    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let span = span!(Level::INFO, "execute", operation = %operation_name.unwrap_or_default());
        let now = Instant::now();

        async move {
            debug!("started execution");
            let response = next.run(ctx, operation_name).await;
            debug!(
                latency = format_args!("{} ms", now.elapsed().as_millis()),
                "finished execution",
            );

            response
        }
        .instrument(span)
        .await
    }

    async fn resolve(
        &self,
        ctx: &ExtensionContext<'_>,
        info: ResolveInfo<'_>,
        next: NextResolve<'_>,
    ) -> ServerResult<Option<Value>> {
        let path_node = info.path_node.to_string();
        if path_node.starts_with("__schema") {
            return next.run(ctx, info).await;
        }

        let span = span!(
            Level::INFO,
            "field",
            path = %path_node,
            parent_type = %info.parent_type,
            return_type = %info.return_type,
        );
        let now = Instant::now();

        async move {
            debug!("started resolution");
            let result = next.run(ctx, info).await;
            debug!(
                latency = format_args!("{} ms", now.elapsed().as_millis()),
                "finished resolution",
            );

            if let Err(ref err) = result {
                error!(error = %err.message, locations = ?err.locations, path = ?err.path);
            }

            result
        }
        .instrument(span)
        .await
    }
}
