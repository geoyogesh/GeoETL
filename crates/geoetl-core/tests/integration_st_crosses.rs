//! Integration tests for `ST_Crosses` spatial predicate

use datafusion::arrow::array::{BooleanArray, RecordBatch};
use datafusion::prelude::*;
use geoetl_operations::register_spatial_udfs;

fn setup_ctx() -> SessionContext {
    let ctx = SessionContext::new();
    register_spatial_udfs(&ctx).unwrap();
    ctx
}

#[tokio::test]
async fn test_st_crosses_two_lines() {
    let ctx = setup_ctx();

    // Two lines that cross each other
    let sql = r"
        SELECT
            ST_Crosses(
                ST_GeomFromText('LINESTRING(0 0, 10 10)'),
                ST_GeomFromText('LINESTRING(0 10, 10 0)')
            ) as crosses
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(result.value(0), "Crossing lines should cross");
}

#[tokio::test]
async fn test_st_crosses_parallel_lines() {
    let ctx = setup_ctx();

    // Two parallel lines
    let sql = r"
        SELECT
            ST_Crosses(
                ST_GeomFromText('LINESTRING(0 0, 10 0)'),
                ST_GeomFromText('LINESTRING(0 5, 10 5)')
            ) as crosses
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(!result.value(0), "Parallel lines should not cross");
}

#[tokio::test]
async fn test_st_crosses_line_polygon() {
    let ctx = setup_ctx();

    // Line crossing through a polygon
    let sql = r"
        SELECT
            ST_Crosses(
                ST_GeomFromText('LINESTRING(-5 5, 15 5)'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as crosses
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
        "Line crossing through polygon should cross"
    );
}

#[tokio::test]
async fn test_st_crosses_with_filter() {
    let ctx = setup_ctx();

    // Filter line pairs to find those that cross
    let sql = r"
        WITH line_pairs AS (
            SELECT 'crossing' as name,
                   ST_GeomFromText('LINESTRING(0 0, 10 10)') as geom_a,
                   ST_GeomFromText('LINESTRING(0 10, 10 0)') as geom_b
            UNION ALL
            SELECT 'parallel',
                   ST_GeomFromText('LINESTRING(0 0, 10 0)'),
                   ST_GeomFromText('LINESTRING(0 5, 10 5)')
            UNION ALL
            SELECT 'collinear',
                   ST_GeomFromText('LINESTRING(0 0, 5 0)'),
                   ST_GeomFromText('LINESTRING(3 0, 10 0)')
        )
        SELECT name FROM line_pairs
        WHERE ST_Crosses(geom_a, geom_b)
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(total_rows, 1, "Only the crossing line pair should cross");
}
