//! Integration tests for `ST_Within` spatial predicate

use datafusion::arrow::array::{BooleanArray, RecordBatch};
use datafusion::prelude::*;
use geoetl_operations::register_spatial_udfs;

fn setup_ctx() -> SessionContext {
    let ctx = SessionContext::new();
    register_spatial_udfs(&ctx).unwrap();
    ctx
}

#[tokio::test]
async fn test_st_within_point_in_polygon() {
    let ctx = setup_ctx();

    // Test if point is within polygon
    let sql = r"
        SELECT
            ST_Within(
                ST_GeomFromText('POINT(5 5)'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as within
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(result.value(0), "Point should be within polygon");
}

#[tokio::test]
async fn test_st_within_point_outside_polygon() {
    let ctx = setup_ctx();

    // Test if point outside is not within polygon
    let sql = r"
        SELECT
            ST_Within(
                ST_GeomFromText('POINT(15 15)'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as within
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(
        !result.value(0),
        "Point outside should not be within polygon"
    );
}

#[tokio::test]
async fn test_st_within_polygon_in_polygon() {
    let ctx = setup_ctx();

    // Small polygon within large polygon
    let sql = r"
        SELECT
            ST_Within(
                ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))'),
                ST_GeomFromText('POLYGON((0 0, 20 0, 20 20, 0 20, 0 0))')
            ) as within
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(
        result.value(0),
        "Small polygon should be within large polygon"
    );
}

#[tokio::test]
async fn test_st_within_inverse_of_contains() {
    let ctx = setup_ctx();

    // ST_Within(A, B) should equal ST_Contains(B, A)
    let sql = r"
        SELECT
            ST_Within(
                ST_GeomFromText('POINT(5 5)'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as within,
            ST_Contains(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POINT(5 5)')
            ) as contains
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let within_result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    let contains_result = batches[0]
        .column(1)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert_eq!(
        within_result.value(0),
        contains_result.value(0),
        "ST_Within(A, B) should equal ST_Contains(B, A)"
    );
}

#[tokio::test]
async fn test_st_within_with_filter() {
    let ctx = setup_ctx();

    // Create a table with points and filter those within a region
    let sql = r"
        WITH points AS (
            SELECT 'inside' as name, ST_GeomFromText('POINT(5 5)') as geom
            UNION ALL
            SELECT 'outside', ST_GeomFromText('POINT(50 50)')
        )
        SELECT name FROM points
        WHERE ST_Within(geom, ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'))
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(
        total_rows, 1,
        "Only the point inside should be within the polygon"
    );
}
