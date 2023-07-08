//! This module defines the routes of the API.

use crate::api::schema::{
    ApiTags, DeleteResponse, GetGraphResponse, GetRecordsResponse, GetWholeTableResponse,
    NodeIdsQuery, Pagination, PaginationQuery, PostResponse, SimilarityNodeQuery, SubgraphIdQuery,
};
use crate::model::core::{
    Entity, Entity2D, EntityMetadata, KnowledgeCuration, RecordResponse, Relation,
    RelationMetadata, Subgraph,
};
use crate::model::graph::Graph;
use log::{debug, info, warn};
use poem::web::Data;
use poem_openapi::{param::Path, param::Query, payload::Json, OpenApi};
use std::sync::Arc;
use validator::Validate;

pub struct BiomedgpsApi;

#[OpenApi]
impl BiomedgpsApi {
    /// Call `/api/v1/entity-metadata` with query params to fetch all entity metadata.
    #[oai(
        path = "/api/v1/entity-metadata",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchEntityMetadata"
    )]
    async fn fetch_entity_metadata(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
    ) -> GetWholeTableResponse<EntityMetadata> {
        let pool_arc = pool.clone();

        match EntityMetadata::get_entity_metadata(&pool_arc).await {
            Ok(entity_metadata) => GetWholeTableResponse::Ok(Json(entity_metadata)),
            Err(e) => {
                let err = format!("Failed to fetch entity metadata: {}", e);
                warn!("{}", err);
                return GetWholeTableResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/relation-metadata` with query params to fetch all relation metadata.
    #[oai(
        path = "/api/v1/relation-metadata",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchRelationMetadata"
    )]
    async fn fetch_relation_metadata(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
    ) -> GetWholeTableResponse<RelationMetadata> {
        let pool_arc = pool.clone();

        match RelationMetadata::get_relation_metadata(&pool_arc).await {
            Ok(relation_metadata) => GetWholeTableResponse::Ok(Json(relation_metadata)),
            Err(e) => {
                let err = format!("Failed to fetch relation metadata: {}", e);
                warn!("{}", err);
                return GetWholeTableResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/entities` with query params to fetch entities.
    #[oai(
        path = "/api/v1/entities",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchEntities"
    )]
    async fn fetch_entities(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        page: Query<Option<u64>>,
        page_size: Query<Option<u64>>,
        query_str: Query<Option<String>>,
    ) -> GetRecordsResponse<Entity> {
        let pool_arc = pool.clone();
        let page = page.0;
        let page_size = page_size.0;

        let query_str = match query_str.0 {
            Some(query_str) => query_str,
            None => {
                warn!("Query string is empty.");
                "".to_string()
            }
        };

        let query = if query_str == "" {
            None
        } else {
            debug!("Query string: {}", &query_str);
            // Parse query string as json
            match serde_json::from_str(&query_str) {
                Ok(query) => Some(query),
                Err(e) => {
                    let err = format!("Failed to parse query string: {}", e);
                    warn!("{}", err);
                    return GetRecordsResponse::bad_request(err);
                }
            }
        };

        match RecordResponse::<Entity>::get_records(
            &pool_arc,
            "biomedgps_entity",
            &query,
            page,
            page_size,
            Some("id ASC"),
        )
        .await
        {
            Ok(entities) => GetRecordsResponse::Ok(Json(entities)),
            Err(e) => {
                let err = format!("Failed to fetch entities: {}", e);
                warn!("{}", err);
                return GetRecordsResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/curated-knowledges` with query params to fetch curated knowledges.
    #[oai(
        path = "/api/v1/curated-knowledges",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchCuratedKnowledges"
    )]
    async fn fetch_curated_knowledges(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        page: Query<Option<u64>>,
        page_size: Query<Option<u64>>,
        query_str: Query<Option<String>>,
    ) -> GetRecordsResponse<KnowledgeCuration> {
        let pool_arc = pool.clone();
        let page = page.0;
        let page_size = page_size.0;

        match PaginationQuery::new(page.clone(), page_size.clone(), query_str.0.clone()) {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to parse query string: {}", e);
                warn!("{}", err);
                return GetRecordsResponse::bad_request(err);
            }
        }

        let query_str = match query_str.0 {
            Some(query_str) => query_str,
            None => {
                warn!("Query string is empty.");
                "".to_string()
            }
        };

        let query = if query_str == "" {
            None
        } else {
            debug!("Query string: {}", &query_str);
            // Parse query string as json
            match serde_json::from_str(&query_str) {
                Ok(query) => Some(query),
                Err(e) => {
                    let err = format!("Failed to parse query string: {}", e);
                    warn!("{}", err);
                    return GetRecordsResponse::bad_request(err);
                }
            }
        };

        match RecordResponse::<KnowledgeCuration>::get_records(
            &pool_arc,
            "biomedgps_knowledge_curation",
            &query,
            page,
            page_size,
            Some("id ASC"),
        )
        .await
        {
            Ok(entities) => GetRecordsResponse::Ok(Json(entities)),
            Err(e) => {
                let err = format!("Failed to fetch curated knowledges: {}", e);
                warn!("{}", err);
                return GetRecordsResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/curated-knowledges` with payload to create a curated knowledge.
    #[oai(
        path = "/api/v1/curated-knowledges",
        method = "post",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "postCuratedKnowledge"
    )]
    async fn post_curated_knowledge(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        payload: Json<KnowledgeCuration>,
    ) -> PostResponse<KnowledgeCuration> {
        let pool_arc = pool.clone();
        let payload = payload.0;

        match payload.validate() {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to validate payload: {}", e);
                warn!("{}", err);
                return PostResponse::bad_request(err);
            }
        };

        match payload.insert(&pool_arc).await {
            Ok(kc) => PostResponse::Created(Json(kc)),
            Err(e) => {
                let err = format!("Failed to insert curated knowledge: {}", e);
                warn!("{}", err);
                return PostResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/curated-knowledges/:id` with payload to create a curated knowledge.
    #[oai(
        path = "/api/v1/curated-knowledges/:id",
        method = "put",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "putCuratedKnowledge"
    )]
    async fn put_curated_knowledge(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        payload: Json<KnowledgeCuration>,
        id: Path<i64>,
    ) -> PostResponse<KnowledgeCuration> {
        let pool_arc = pool.clone();
        let payload = payload.0;
        let id = id.0;

        if id < 0 {
            let err = format!("Invalid id: {}", id);
            warn!("{}", err);
            return PostResponse::bad_request(err);
        }

        match payload.validate() {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to validate payload: {}", e);
                warn!("{}", err);
                return PostResponse::bad_request(err);
            }
        };

        match payload.update(&pool_arc, id).await {
            Ok(kc) => PostResponse::Created(Json(kc)),
            Err(e) => {
                let err = format!("Failed to insert curated knowledge: {}", e);
                warn!("{}", err);
                return PostResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/curated-knowledges/:id` with payload to delete a curated knowledge.
    #[oai(
        path = "/api/v1/curated-knowledges/:id",
        method = "delete",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "deleteCuratedKnowledge"
    )]
    async fn delete_curated_knowledge(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        id: Path<i64>,
    ) -> DeleteResponse {
        let pool_arc = pool.clone();
        let id = id.0;

        if id < 0 {
            let err = format!("Invalid id: {}", id);
            warn!("{}", err);
            return DeleteResponse::bad_request(err);
        }

        match KnowledgeCuration::delete(&pool_arc, id).await {
            Ok(_) => DeleteResponse::no_content(),
            Err(e) => {
                let err = format!("Failed to delete curated knowledge: {}", e);
                warn!("{}", err);
                DeleteResponse::not_found(err)
            }
        }
    }

    /// Call `/api/v1/relations` with query params to fetch relations.
    #[oai(
        path = "/api/v1/relations",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchRelations"
    )]
    async fn fetch_relations(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        page: Query<Option<u64>>,
        page_size: Query<Option<u64>>,
        query_str: Query<Option<String>>,
    ) -> GetRecordsResponse<Relation> {
        let pool_arc = pool.clone();
        let page = page.0;
        let page_size = page_size.0;

        match PaginationQuery::new(page.clone(), page_size.clone(), query_str.0.clone()) {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to parse query string: {}", e);
                warn!("{}", err);
                return GetRecordsResponse::bad_request(err);
            }
        };

        let query_str = match query_str.0 {
            Some(query_str) => query_str,
            None => {
                warn!("Query string is empty.");
                "".to_string()
            }
        };

        let query = if query_str == "" {
            None
        } else {
            debug!("Query string: {}", &query_str);
            // Parse query string as json
            match serde_json::from_str(&query_str) {
                Ok(query) => Some(query),
                Err(e) => {
                    let err = format!("Failed to parse query string: {}", e);
                    warn!("{}", err);
                    return GetRecordsResponse::bad_request(err);
                }
            }
        };

        match RecordResponse::<Relation>::get_records(
            &pool_arc,
            "biomedgps_relation",
            &query,
            page,
            page_size,
            Some("id ASC"),
        )
        .await
        {
            Ok(entities) => GetRecordsResponse::Ok(Json(entities)),
            Err(e) => {
                let err = format!("Failed to fetch relations: {}", e);
                warn!("{}", err);
                return GetRecordsResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/entity2d` with query params to fetch entity2d.
    #[oai(
        path = "/api/v1/entity2d",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchEntity2d"
    )]
    async fn fetch_entity2d(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        page: Query<Option<u64>>,
        page_size: Query<Option<u64>>,
        query_str: Query<Option<String>>,
    ) -> GetRecordsResponse<Entity2D> {
        let pool_arc = pool.clone();
        let page = page.0;
        let page_size = page_size.0;

        match PaginationQuery::new(page.clone(), page_size.clone(), query_str.0.clone()) {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to parse query string: {}", e);
                warn!("{}", err);
                return GetRecordsResponse::bad_request(err);
            }
        }

        let query_str = match query_str.0 {
            Some(query_str) => query_str,
            None => {
                warn!("Query string is empty.");
                "".to_string()
            }
        };

        let query = if query_str == "" {
            None
        } else {
            debug!("Query string: {}", &query_str);
            // Parse query string as json
            match serde_json::from_str(&query_str) {
                Ok(query) => Some(query),
                Err(e) => {
                    let err = format!("Failed to parse query string: {}", e);
                    warn!("{}", err);
                    return GetRecordsResponse::bad_request(err);
                }
            }
        };

        match RecordResponse::<Entity2D>::get_records(
            &pool_arc,
            "biomedgps_entity2d",
            &query,
            page,
            page_size,
            Some("embedding_id ASC"),
        )
        .await
        {
            Ok(entities) => GetRecordsResponse::Ok(Json(entities)),
            Err(e) => {
                let err = format!("Failed to fetch entity2ds: {}", e);
                warn!("{}", err);
                return GetRecordsResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/subgraphs` with query params to fetch subgraphs.
    #[oai(
        path = "/api/v1/subgraphs",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchSubgraphs"
    )]
    async fn fetch_subgraphs(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        page: Query<Option<u64>>,
        page_size: Query<Option<u64>>,
        query_str: Query<Option<String>>,
    ) -> GetRecordsResponse<Subgraph> {
        let pool_arc = pool.clone();
        let page = page.0;
        let page_size = page_size.0;

        match PaginationQuery::new(page.clone(), page_size.clone(), query_str.0.clone()) {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to parse query string: {}", e);
                warn!("{}", err);
                return GetRecordsResponse::bad_request(err);
            }
        }

        let query_str = match query_str.0 {
            Some(query_str) => query_str,
            None => {
                warn!("Query string is empty.");
                "".to_string()
            }
        };

        let query = if query_str == "" {
            None
        } else {
            debug!("Query string: {}", &query_str);
            // Parse query string as json
            match serde_json::from_str(&query_str) {
                Ok(query) => Some(query),
                Err(e) => {
                    let err = format!("Failed to parse query string: {}", e);
                    warn!("{}", err);
                    return GetRecordsResponse::bad_request(err);
                }
            }
        };

        match RecordResponse::<Subgraph>::get_records(
            &pool_arc,
            "biomedgps_subgraph",
            &query,
            page,
            page_size,
            Some("created_time DESC"),
        )
        .await
        {
            Ok(entities) => GetRecordsResponse::Ok(Json(entities)),
            Err(e) => {
                let err = format!("Failed to fetch subgraphs: {}", e);
                warn!("{}", err);
                return GetRecordsResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/subgraphs` with payload to create a subgraph.
    #[oai(
        path = "/api/v1/subgraphs",
        method = "post",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "postSubgraph"
    )]
    async fn post_subgraph(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        payload: Json<Subgraph>,
    ) -> PostResponse<Subgraph> {
        let pool_arc = pool.clone();
        let payload = payload.0;

        match payload.validate() {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to validate subgraph: {}", e);
                warn!("{}", err);
                return PostResponse::bad_request(err);
            }
        };

        match payload.insert(&pool_arc).await {
            Ok(kc) => PostResponse::Created(Json(kc)),
            Err(e) => {
                let err = format!("Failed to insert curated knowledge: {}", e);
                warn!("{}", err);
                return PostResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/subgraphs/:id` with payload to update a subgraph.
    #[oai(
        path = "/api/v1/subgraphs/:id",
        method = "put",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "putSubgraph"
    )]
    async fn put_subgraph(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        id: Path<String>,
        payload: Json<Subgraph>,
    ) -> PostResponse<Subgraph> {
        let pool_arc = pool.clone();
        let id = id.0;
        let payload = payload.0;

        match SubgraphIdQuery::new(&id) {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to parse subgraph id: {}", e);
                warn!("{}", err);
                return PostResponse::bad_request(err);
            }
        }

        match payload.validate() {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to validate subgraph: {}", e);
                warn!("{}", err);
                return PostResponse::bad_request(err);
            }
        }

        match payload.update(&pool_arc, &id).await {
            Ok(kc) => PostResponse::Created(Json(kc)),
            Err(e) => {
                let err = format!("Failed to update subgraph: {}", e);
                warn!("{}", err);
                return PostResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/subgraphs/:id` with payload to create subgraph.
    #[oai(
        path = "/api/v1/subgraphs/:id",
        method = "delete",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "deleteSubgraph"
    )]
    async fn delete_subgraph(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        id: Path<String>,
    ) -> DeleteResponse {
        let pool_arc = pool.clone();
        let id = id.0;

        match SubgraphIdQuery::new(&id) {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to validate subgraph id: {}", e);
                warn!("{}", err);
                return DeleteResponse::bad_request(err);
            }
        }

        match Subgraph::delete(&pool_arc, &id).await {
            Ok(_) => DeleteResponse::NoContent,
            Err(e) => {
                let err = format!("Failed to delete a subgraph: {}", e);
                warn!("{}", err);
                DeleteResponse::not_found(err)
            }
        }
    }

    /// Call `/api/v1/nodes` with query params to fetch nodes.
    #[oai(
        path = "/api/v1/nodes",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchNodes"
    )]
    async fn fetch_nodes(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        node_ids: Query<String>,
    ) -> GetGraphResponse {
        let pool_arc = pool.clone();
        let node_ids = node_ids.0;

        match NodeIdsQuery::new(&node_ids) {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to validate node ids: {}", e);
                warn!("{}", err);
                return GetGraphResponse::bad_request(err);
            }
        };

        let mut graph = Graph::new();

        if node_ids == "" {
            return GetGraphResponse::Ok(Json(graph));
        }

        let node_ids: Vec<&str> = node_ids.split(",").collect();
        match graph.fetch_nodes_by_ids(&pool_arc, &node_ids).await {
            Ok(graph) => GetGraphResponse::Ok(Json(graph.to_owned().get_graph(None).unwrap())),
            Err(e) => {
                let err = format!("Failed to fetch nodes: {}", e);
                warn!("{}", err);
                return GetGraphResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/auto-connect-nodes` with query params to fetch edges which connect the input nodes.
    #[oai(
        path = "/api/v1/auto-connect-nodes",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchEdgesAutoConnectNodes"
    )]
    async fn fetch_edges_auto_connect_nodes(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        node_ids: Query<String>,
    ) -> GetGraphResponse {
        let pool_arc = pool.clone();
        let node_ids = node_ids.0;

        match NodeIdsQuery::new(&node_ids) {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to validate node ids: {}", e);
                warn!("{}", err);
                return GetGraphResponse::bad_request(err);
            }
        };

        let mut graph = Graph::new();

        if node_ids == "" {
            return GetGraphResponse::Ok(Json(graph));
        }

        let node_ids: Vec<&str> = node_ids.split(",").collect();
        match graph.auto_connect_nodes(&pool_arc, &node_ids).await {
            Ok(graph) => GetGraphResponse::Ok(Json(graph.to_owned().get_graph(None).unwrap())),
            Err(e) => {
                let err = format!("Failed to fetch nodes: {}", e);
                warn!("{}", err);
                return GetGraphResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/one-step-linked-nodes` with query params to fetch linked nodes with one step.
    #[oai(
        path = "/api/v1/one-step-linked-nodes",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchOneStepLinkedNodes"
    )]
    async fn fetch_one_step_linked_nodes(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        page: Query<Option<u64>>,
        page_size: Query<Option<u64>>,
        query_str: Query<Option<String>>,
    ) -> GetGraphResponse {
        let pool_arc = pool.clone();
        let page = page.0;
        let page_size = page_size.0;

        match PaginationQuery::new(page.clone(), page_size.clone(), query_str.0.clone()) {
            Ok(_) => {}
            Err(e) => {
                let err = format!("Failed to parse query string: {}", e);
                warn!("{}", err);
                return GetGraphResponse::bad_request(err);
            }
        };

        let query_str = match query_str.0 {
            Some(query_str) => query_str,
            None => {
                warn!("Query string is empty.");
                "".to_string()
            }
        };

        let query = if query_str == "" {
            None
        } else {
            debug!("Query string: {}", &query_str);
            // Parse query string as json
            match serde_json::from_str(&query_str) {
                Ok(query) => Some(query),
                Err(e) => {
                    let err = format!("Failed to parse query string: {}", e);
                    warn!("{}", err);
                    return GetGraphResponse::bad_request(err);
                }
            }
        };

        let mut graph = Graph::new();
        match graph
            .fetch_linked_nodes(&pool_arc, &query, page, page_size, None)
            .await
        {
            Ok(graph) => GetGraphResponse::Ok(Json(graph.to_owned().get_graph(None).unwrap())),
            Err(e) => {
                let err = format!("Failed to fetch linked nodes: {}", e);
                warn!("{}", err);
                return GetGraphResponse::bad_request(err);
            }
        }
    }

    /// Call `/api/v1/similarity-nodes` with query params to fetch similarity nodes.
    #[oai(
        path = "/api/v1/similarity-nodes",
        method = "get",
        tag = "ApiTags::KnowledgeGraph",
        operation_id = "fetchSimilarityNodes"
    )]
    async fn fetch_similarity_nodes(
        &self,
        pool: Data<&Arc<sqlx::PgPool>>,
        node_id: Query<String>,
        query_str: Query<Option<String>>,
        topk: Query<Option<u64>>,
    ) -> GetGraphResponse {
        let pool_arc = pool.clone();

        match SimilarityNodeQuery::new(&node_id.0, &query_str.0, topk.0) {
            Ok(query) => query,
            Err(e) => {
                let err = format!("Failed to parse query string: {}", e);
                warn!("{}", err);
                return GetGraphResponse::bad_request(err);
            }
        };

        let query_str = match query_str.0 {
            Some(query_str) => query_str,
            None => {
                warn!("Query string is empty.");
                "".to_string()
            }
        };

        let topk = topk.0;

        let query = if query_str == "" {
            None
        } else {
            debug!("Query string: {}", &query_str);
            // Parse query string as json
            match serde_json::from_str(&query_str) {
                Ok(query) => Some(query),
                Err(e) => {
                    let err = format!("Failed to parse query string: {}", e);
                    warn!("{}", err);
                    return GetGraphResponse::bad_request(err);
                }
            }
        };

        let mut graph = Graph::new();
        match graph
            .fetch_similarity_nodes(&pool_arc, &node_id, &query, topk)
            .await
        {
            Ok(graph) => GetGraphResponse::Ok(Json(graph.to_owned().get_graph(None).unwrap())),
            Err(e) => {
                let err = format!("Failed to fetch similarity nodes: {}", e);
                warn!("{}", err);
                return GetGraphResponse::bad_request(err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::graph::Node;
    use crate::{init_log, kv2urlstr, setup_test_db};
    use log::{debug, error};
    use poem::middleware::{AddData, AddDataEndpoint};
    use poem::test::TestClient;
    use poem::{
        http::{StatusCode, Uri},
        Endpoint, EndpointExt, Request, Route,
    };
    use poem_openapi::OpenApiService;
    use sqlx::{Pool, Postgres};

    async fn init_app() -> AddDataEndpoint<Route, Arc<Pool<Postgres>>> {
        init_log();
        let pool = setup_test_db().await;

        let arc_pool = Arc::new(pool);
        let shared_rb = AddData::new(arc_pool.clone());
        let service = OpenApiService::new(BiomedgpsApi, "BioMedGPS", "v0.1.0");
        let app = Route::new().nest("/", service).with(shared_rb);
        app
    }

    #[tokio::test]
    async fn test_fetch_entities() {
        let app = init_app().await;
        let cli = TestClient::new(app);

        let resp = cli.get("/api/v1/entities").send().await;
        resp.assert_status_is_ok();

        let json = resp.json().await;
        let entity_records = json.value().deserialize::<RecordResponse<Entity>>();
        assert!(entity_records.records.len() > 0);
        let resp = cli.get("/api/v1/entities?page=1&page_size=10").send().await;
        resp.assert_status_is_ok();

        let json = resp.json().await;
        let entity_records = json.value().deserialize::<RecordResponse<Entity>>();
        assert!(entity_records.records.len() == 10);

        let query_json_str = r#"{"operator": "=", "field": "id", "value": "DOID:2022"}"#;
        let query_str = kv2urlstr("query_str", &query_json_str.to_string());
        debug!("Query string: {}", query_str);

        let resp = cli
            .get(format!(
                "/api/v1/entities?page=1&page_size=10&{}",
                query_str
            ))
            .send()
            .await;
        resp.assert_status_is_ok();

        let json = resp.json().await;
        let entity_records = json.value().deserialize::<RecordResponse<Entity>>();
        assert!(entity_records.records.len() == 1);

        let query_json_str = r#"{
            "operator": "and", "items": [
                {"operator": "=", "field": "id", "value": "DOID:2022"},
                {"operator": "=", "field": "label", "value": "Disease"}
            ]
        }"#;
        let query_str = kv2urlstr("query_str", &query_json_str.to_string());
        debug!("Query string: {}", query_str);

        let resp = cli
            .get(format!(
                "/api/v1/entities?page=1&page_size=10&{}",
                query_str
            ))
            .send()
            .await;
        resp.assert_status_is_ok();

        let json = resp.json().await;
        let entity_records = json.value().deserialize::<RecordResponse<Entity>>();
        assert!(entity_records.records.len() == 1);

        let query_json_str = r#"{
            "operator": "and", "items": [
                {"operator": "=", "field": "id", "value": "NOT-FOUND:2022"},
                {"operator": "=", "field": "label", "value": "NOT-FOUND"}
            ]
        }"#;
        let query_str = kv2urlstr("query_str", &query_json_str.to_string());
        debug!("Query string: {}", query_str);

        let resp = cli
            .get(format!(
                "/api/v1/entities?page=1&page_size=10&{}",
                query_str
            ))
            .send()
            .await;
        resp.assert_status_is_ok();

        let json = resp.json().await;
        let entity_records = json.value().deserialize::<RecordResponse<Entity>>();
        assert!(entity_records.records.len() == 0);
    }

    #[tokio::test]
    async fn test_fetch_similarity_nodes() {
        let app = init_app().await;
        let cli = TestClient::new(app);

        let resp = cli.get("/api/v1/similarity-nodes").send().await;
        resp.assert_status(StatusCode::BAD_REQUEST);

        let resp = cli
            .get("/api/v1/similarity-nodes?node_id=Chemical::MESH:C000601183")
            .send()
            .await;
        let json = resp.json().await;
        let nodes = json.value().object().get("nodes");
        nodes.assert_not_null();

        // TODO: Cannot deserialize Graph, because we cannot rename the field lineWidth to line_width when deserializing.
        // The poem-openapi crate does not support to rename a field when deserializing.
        //
        // let mut records = json.value().deserialize::<Graph>();
        // assert!(records.get_nodes().len() == 10);
    }
}
