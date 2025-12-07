//! Integration tests for `ST_Disjoint` spatial predicate

use datafusion::arrow::array::{BooleanArray, RecordBatch};
use datafusion::prelude::*;
use geoetl_operations::register_spatial_udfs;

fn setup_ctx() -> SessionContext {
    let ctx = SessionContext::new();
    register_spatial_udfs(&ctx).unwrap();
    ctx
}

#[tokio::test]
async fn test_st_disjoint_separate_polygons() {
    let ctx = setup_ctx();

    // Two completely separate polygons
    let sql = r"
        SELECT
            ST_Disjoint(
                ST_GeomFromText('POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))'),
                ST_GeomFromText('POLYGON((10 10, 15 10, 15 15, 10 15, 10 10))')
            ) as disjoint
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(result.value(0), "Separate polygons should be disjoint");
}

#[tokio::test]
async fn test_st_disjoint_overlapping_polygons() {
    let ctx = setup_ctx();

    // Two overlapping polygons
    let sql = r"
        SELECT
            ST_Disjoint(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))')
            ) as disjoint
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
        "Overlapping polygons should not be disjoint"
    );
}

#[tokio::test]
async fn test_st_disjoint_inverse_of_intersects() {
    let ctx = setup_ctx();

    // Verify disjoint is inverse of intersects
    let sql = r"
        SELECT
            ST_Disjoint(
                ST_GeomFromText('POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))'),
                ST_GeomFromText('POLYGON((10 10, 15 10, 15 15, 10 15, 10 10))')
            ) as disjoint,
            ST_Intersects(
                ST_GeomFromText('POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))'),
                ST_GeomFromText('POLYGON((10 10, 15 10, 15 15, 10 15, 10 10))')
            ) as intersects
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let disjoint = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    let intersects = batches[0]
        .column(1)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert_ne!(
        disjoint.value(0),
        intersects.value(0),
        "Disjoint should be inverse of intersects"
    );
}

#[tokio::test]
async fn test_st_disjoint_with_filter() {
    let ctx = setup_ctx();

    // Filter to find disjoint geometry pairs
    let sql = r"
        WITH geom_pairs AS (
            SELECT 'separate' as name,
                   ST_GeomFromText('POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))') as geom_a,
                   ST_GeomFromText('POLYGON((10 10, 15 10, 15 15, 10 15, 10 10))') as geom_b
            UNION ALL
            SELECT 'overlapping',
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                   ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))')
            UNION ALL
            SELECT 'touching',
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                   ST_GeomFromText('POLYGON((10 0, 20 0, 20 10, 10 10, 10 0))')
        )
        SELECT name FROM geom_pairs
        WHERE ST_Disjoint(geom_a, geom_b)
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(total_rows, 1, "Only the separate pair should be disjoint");
}
