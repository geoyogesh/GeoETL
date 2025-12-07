//! Integration tests for `ST_CoveredBy` spatial predicate

use datafusion::arrow::array::{BooleanArray, RecordBatch};
use datafusion::prelude::*;
use geoetl_operations::register_spatial_udfs;

fn setup_ctx() -> SessionContext {
    let ctx = SessionContext::new();
    register_spatial_udfs(&ctx).unwrap();
    ctx
}

#[tokio::test]
async fn test_st_covered_by_point_inside() {
    let ctx = setup_ctx();

    // Point inside polygon is covered by it
    let sql = r"
        SELECT
            ST_CoveredBy(
                ST_GeomFromText('POINT(5 5)'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as covered_by
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
        "Point inside polygon should be covered by polygon"
    );
}

#[tokio::test]
async fn test_st_covered_by_point_on_boundary() {
    let ctx = setup_ctx();

    // Point on boundary is covered by polygon
    let sql = r"
        SELECT
            ST_CoveredBy(
                ST_GeomFromText('POINT(10 5)'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as covered_by
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
        "Point on boundary should be covered by polygon"
    );
}

#[tokio::test]
async fn test_st_covered_by_point_outside() {
    let ctx = setup_ctx();

    // Point outside polygon is not covered
    let sql = r"
        SELECT
            ST_CoveredBy(
                ST_GeomFromText('POINT(20 20)'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as covered_by
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
        "Point outside polygon should not be covered"
    );
}

#[tokio::test]
async fn test_st_covered_by_inverse_of_covers() {
    let ctx = setup_ctx();

    // Verify ST_CoveredBy(A, B) == ST_Covers(B, A)
    let sql = r"
        SELECT
            ST_CoveredBy(
                ST_GeomFromText('POINT(5 5)'),
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            ) as covered_by,
            ST_Covers(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POINT(5 5)')
            ) as covers
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let covered_by = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    let covers = batches[0]
        .column(1)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert_eq!(
        covered_by.value(0),
        covers.value(0),
        "ST_CoveredBy(A, B) should equal ST_Covers(B, A)"
    );
}

#[tokio::test]
async fn test_st_covered_by_with_filter() {
    let ctx = setup_ctx();

    // Filter to find geometries covered by a container
    let sql = r"
        WITH geom_pairs AS (
            SELECT 'inside' as name,
                   ST_GeomFromText('POINT(5 5)') as geom,
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))') as container
            UNION ALL
            SELECT 'on_boundary',
                   ST_GeomFromText('POINT(10 5)'),
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
            UNION ALL
            SELECT 'outside',
                   ST_GeomFromText('POINT(20 20)'),
                   ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
        )
        SELECT name FROM geom_pairs
        WHERE ST_CoveredBy(geom, container)
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(
        total_rows, 2,
        "Both inside and on_boundary should be covered by container"
    );
}
