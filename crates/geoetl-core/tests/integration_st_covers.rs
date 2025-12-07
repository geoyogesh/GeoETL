//! Integration tests for `ST_Covers` spatial predicate

use datafusion::arrow::array::{BooleanArray, RecordBatch};
use datafusion::prelude::*;
use geoetl_operations::register_spatial_udfs;

fn setup_ctx() -> SessionContext {
    let ctx = SessionContext::new();
    register_spatial_udfs(&ctx).unwrap();
    ctx
}

#[tokio::test]
async fn test_st_covers_point_inside() {
    let ctx = setup_ctx();

    // Polygon covers point inside it
    let sql = r"
        SELECT
            ST_Covers(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POINT(5 5)')
            ) as covers
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(result.value(0), "Polygon should cover point inside it");
}

#[tokio::test]
async fn test_st_covers_point_on_boundary() {
    let ctx = setup_ctx();

    // Polygon covers point on its boundary
    let sql = r"
        SELECT
            ST_Covers(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POINT(10 5)')
            ) as covers
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
        "Polygon should cover point on its boundary"
    );
}

#[tokio::test]
async fn test_st_covers_point_outside() {
    let ctx = setup_ctx();

    // Polygon does not cover point outside
    let sql = r"
        SELECT
            ST_Covers(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POINT(20 20)')
            ) as covers
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(!result.value(0), "Polygon should not cover point outside");
}

#[tokio::test]
async fn test_st_covers_line_on_boundary() {
    let ctx = setup_ctx();

    // Polygon covers line on its boundary
    let sql = r"
        SELECT
            ST_Covers(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('LINESTRING(0 0, 10 0)')
            ) as covers
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(result.value(0), "Polygon should cover line on its boundary");
}

#[tokio::test]
async fn test_st_covers_with_filter() {
    let ctx = setup_ctx();

    // Filter to find covered geometries
    let sql = r"
        WITH geom_pairs AS (
            SELECT 'inside' as name,
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))') as container,
                   ST_GeomFromText('POINT(5 5)') as contained
            UNION ALL
            SELECT 'on_boundary',
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                   ST_GeomFromText('POINT(10 5)')
            UNION ALL
            SELECT 'outside',
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                   ST_GeomFromText('POINT(20 20)')
        )
        SELECT name FROM geom_pairs
        WHERE ST_Covers(container, contained)
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(
        total_rows, 2,
        "Both inside and on_boundary should be covered"
    );
}
