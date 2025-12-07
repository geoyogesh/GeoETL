//! Integration tests for `ST_Contains` spatial predicate

use datafusion::arrow::array::{BooleanArray, RecordBatch};
use datafusion::prelude::*;
use geoetl_operations::register_spatial_udfs;

fn setup_ctx() -> SessionContext {
    let ctx = SessionContext::new();
    register_spatial_udfs(&ctx).unwrap();
    ctx
}

#[tokio::test]
async fn test_st_contains_point_in_polygon() {
    let ctx = setup_ctx();

    // Test if polygon contains point
    let sql = r"
        SELECT
            ST_Contains(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POINT(5 5)')
            ) as contains
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let result = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(result.value(0), "Polygon should contain point inside it");
}

#[tokio::test]
async fn test_st_contains_point_outside_polygon() {
    let ctx = setup_ctx();

    // Test if polygon does not contain point outside
    let sql = r"
        SELECT
            ST_Contains(
                ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))'),
                ST_GeomFromText('POINT(15 15)')
            ) as contains
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
        "Polygon should not contain point outside it"
    );
}

#[tokio::test]
async fn test_st_contains_polygon_in_polygon() {
    let ctx = setup_ctx();

    // Large polygon contains small polygon
    let sql = r"
        SELECT
            ST_Contains(
                ST_GeomFromText('POLYGON((0 0, 20 0, 20 20, 0 20, 0 0))'),
                ST_GeomFromText('POLYGON((5 5, 15 5, 15 15, 5 15, 5 5))')
            ) as contains
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
        "Large polygon should contain smaller polygon"
    );
}

#[tokio::test]
async fn test_st_contains_with_filter() {
    let ctx = setup_ctx();

    // Create a table with geometries
    let sql = r"
        WITH polygons AS (
            SELECT 'big' as name, ST_GeomFromText('POLYGON((0 0, 100 0, 100 100, 0 100, 0 0))') as geom
            UNION ALL
            SELECT 'small', ST_GeomFromText('POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))')
        )
        SELECT name FROM polygons
        WHERE ST_Contains(geom, ST_GeomFromText('POINT(50 50)'))
    ";

    let df = ctx.sql(sql).await.unwrap();
    let batches: Vec<RecordBatch> = df.collect().await.unwrap();

    let total_rows: usize = batches.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(total_rows, 1, "Only the big polygon should contain (50,50)");
}
