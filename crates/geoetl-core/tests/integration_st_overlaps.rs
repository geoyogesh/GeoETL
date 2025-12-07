//! Integration tests for `ST_Overlaps` spatial predicate

use datafusion::arrow::array::{BooleanArray, RecordBatch};
use datafusion::prelude::*;
use geoetl_operations::register_spatial_udfs;

fn setup_ctx() -> SessionContext {
    let ctx = SessionContext::new();
    register_spatial_udfs(&ctx).unwrap();
    ctx
}

#[tokio::test]
async fn test_st_overlaps_partial_overlap() {
    let ctx = setup_ctx();

    // Two polygons that partially overlap
    let sql = r"
        SELECT
            ST_Overlaps(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))')
            ) as overlaps
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
        "Partially overlapping polygons should overlap"
    );
}

#[tokio::test]
async fn test_st_overlaps_disjoint() {
    let ctx = setup_ctx();

    // Two disjoint polygons
    let sql = r"
        SELECT
            ST_Overlaps(
                ST_GeomFromText('POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))'),
                ST_GeomFromText('POLYGON((10 10, 15 10, 15 15, 10 15, 10 10))')
            ) as overlaps
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(!result.value(0), "Disjoint polygons should not overlap");
}

#[tokio::test]
async fn test_st_overlaps_containment_not_overlap() {
    let ctx = setup_ctx();

    // One polygon completely contains another - not overlap
    let sql = r"
        SELECT
            ST_Overlaps(
                ST_GeomFromText('POLYGON((0 0, 20 0, 20 20, 0 20, 0 0))'),
                ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))')
            ) as overlaps
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
        "Containment should not be considered overlap"
    );
}

#[tokio::test]
async fn test_st_overlaps_identical_not_overlap() {
    let ctx = setup_ctx();

    // Identical polygons should not overlap (they are equal)
    let sql = r"
        SELECT
            ST_Overlaps(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as overlaps
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
        "Identical polygons should not overlap (they are equal)"
    );
}

#[tokio::test]
async fn test_st_overlaps_with_filter() {
    let ctx = setup_ctx();

    // Create a table with polygon pairs and filter those that overlap
    let sql = r"
        WITH polygon_pairs AS (
            SELECT 'partial' as name,
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))') as geom_a,
                   ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))') as geom_b
            UNION ALL
            SELECT 'disjoint',
                   ST_GeomFromText('POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))'),
                   ST_GeomFromText('POLYGON((10 10, 15 10, 15 15, 10 15, 10 10))')
            UNION ALL
            SELECT 'contained',
                   ST_GeomFromText('POLYGON((0 0, 20 0, 20 20, 0 20, 0 0))'),
                   ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))')
        )
        SELECT name FROM polygon_pairs
        WHERE ST_Overlaps(geom_a, geom_b)
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(
        total_rows, 1,
        "Only the partial overlap case should pass the filter"
    );
}

#[tokio::test]
async fn test_st_overlaps_lines() {
    let ctx = setup_ctx();

    // Two lines that partially overlap along their length
    let sql = r"
        SELECT
            ST_Overlaps(
                ST_GeomFromText('LINESTRING(0 0, 10 0)'),
                ST_GeomFromText('LINESTRING(5 0, 15 0)')
            ) as overlaps
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
        "Partially overlapping lines should overlap"
    );
}
