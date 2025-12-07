//! Integration tests for `ST_Touches` spatial predicate

use datafusion::arrow::array::{BooleanArray, RecordBatch};
use datafusion::prelude::*;
use geoetl_operations::register_spatial_udfs;

fn setup_ctx() -> SessionContext {
    let ctx = SessionContext::new();
    register_spatial_udfs(&ctx).unwrap();
    ctx
}

#[tokio::test]
async fn test_st_touches_adjacent_polygons() {
    let ctx = setup_ctx();

    // Two polygons that share an edge
    let sql = r"
        SELECT
            ST_Touches(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POLYGON((10 0, 20 0, 20 10, 10 10, 10 0))')
            ) as touches
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(result.value(0), "Adjacent polygons should touch");
}

#[tokio::test]
async fn test_st_touches_disjoint_polygons() {
    let ctx = setup_ctx();

    // Two disjoint polygons
    let sql = r"
        SELECT
            ST_Touches(
                ST_GeomFromText('POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))'),
                ST_GeomFromText('POLYGON((10 10, 15 10, 15 15, 10 15, 10 10))')
            ) as touches
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(!result.value(0), "Disjoint polygons should not touch");
}

#[tokio::test]
async fn test_st_touches_overlapping_not_touch() {
    let ctx = setup_ctx();

    // Overlapping polygons don't touch (interiors intersect)
    let sql = r"
        SELECT
            ST_Touches(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))')
            ) as touches
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
        "Overlapping polygons should not touch (interiors intersect)"
    );
}

#[tokio::test]
async fn test_st_touches_point_on_boundary() {
    let ctx = setup_ctx();

    // Point on polygon boundary
    let sql = r"
        SELECT
            ST_Touches(
                ST_GeomFromText('POINT(10 5)'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as touches
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(result.value(0), "Point on boundary should touch polygon");
}

#[tokio::test]
async fn test_st_touches_with_filter() {
    let ctx = setup_ctx();

    // Filter polygon pairs to find those that touch
    let sql = r"
        WITH polygon_pairs AS (
            SELECT 'adjacent' as name,
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))') as geom_a,
                   ST_GeomFromText('POLYGON((10 0, 20 0, 20 10, 10 10, 10 0))') as geom_b
            UNION ALL
            SELECT 'disjoint',
                   ST_GeomFromText('POLYGON((0 0, 5 0, 5 5, 0 5, 0 0))'),
                   ST_GeomFromText('POLYGON((10 10, 15 10, 15 15, 10 15, 10 10))')
            UNION ALL
            SELECT 'overlapping',
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                   ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))')
        )
        SELECT name FROM polygon_pairs
        WHERE ST_Touches(geom_a, geom_b)
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(total_rows, 1, "Only the adjacent polygon pair should touch");
}
