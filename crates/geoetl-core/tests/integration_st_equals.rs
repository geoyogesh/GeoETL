//! Integration tests for `ST_Equals` spatial predicate

use datafusion::arrow::array::{BooleanArray, RecordBatch};
use datafusion::prelude::*;
use geoetl_operations::register_spatial_udfs;

fn setup_ctx() -> SessionContext {
    let ctx = SessionContext::new();
    register_spatial_udfs(&ctx).unwrap();
    ctx
}

#[tokio::test]
async fn test_st_equals_identical_polygons() {
    let ctx = setup_ctx();

    // Two identical polygons
    let sql = r"
        SELECT
            ST_Equals(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as equals
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(result.value(0), "Identical polygons should be equal");
}

#[tokio::test]
async fn test_st_equals_same_polygon_different_order() {
    let ctx = setup_ctx();

    // Same polygon but with different vertex ordering
    let sql = r"
        SELECT
            ST_Equals(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POLYGON((0 0, 0 10, 10 10, 10 0, 0 0))')
            ) as equals
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
        "Same polygon with different ordering should be equal"
    );
}

#[tokio::test]
async fn test_st_equals_different_polygons() {
    let ctx = setup_ctx();

    // Two different polygons
    let sql = r"
        SELECT
            ST_Equals(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POLYGON((0 0, 20 0, 20 20, 0 20, 0 0))')
            ) as equals
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(!result.value(0), "Different polygons should not be equal");
}

#[tokio::test]
async fn test_st_equals_identical_points() {
    let ctx = setup_ctx();

    // Two identical points
    let sql = r"
        SELECT
            ST_Equals(
                ST_GeomFromText('POINT(5 5)'),
                ST_GeomFromText('POINT(5 5)')
            ) as equals
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(result.value(0), "Identical points should be equal");
}

#[tokio::test]
async fn test_st_equals_with_filter() {
    let ctx = setup_ctx();

    // Filter to find equal geometry pairs
    let sql = r"
        WITH geom_pairs AS (
            SELECT 'identical' as name,
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))') as geom_a,
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))') as geom_b
            UNION ALL
            SELECT 'different_size',
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                   ST_GeomFromText('POLYGON((0 0, 20 0, 20 20, 0 20, 0 0))')
            UNION ALL
            SELECT 'different_position',
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                   ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))')
        )
        SELECT name FROM geom_pairs
        WHERE ST_Equals(geom_a, geom_b)
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(total_rows, 1, "Only the identical pair should be equal");
}
