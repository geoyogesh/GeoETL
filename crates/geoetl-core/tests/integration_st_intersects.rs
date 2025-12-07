//! Integration tests for `ST_Intersects` spatial predicate

use datafusion::arrow::array::{BooleanArray, RecordBatch};
use datafusion::prelude::*;
use geoetl_operations::register_spatial_udfs;

fn setup_ctx() -> SessionContext {
    let ctx = SessionContext::new();
    register_spatial_udfs(&ctx).unwrap();
    ctx
}

#[tokio::test]
async fn test_st_intersects_overlapping() {
    let ctx = setup_ctx();

    let result = ctx
        .sql(
            "SELECT ST_Intersects(
                ST_GeomFromText('POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))'),
                ST_GeomFromText('POLYGON((1 1, 3 1, 3 3, 1 3, 1 1))')
            ) as intersects",
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();

    assert_eq!(result.len(), 1);
    let batch = &result[0];
    let intersects = batch
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(intersects.value(0), "Overlapping polygons should intersect");
}

#[tokio::test]
async fn test_st_intersects_disjoint() {
    let ctx = setup_ctx();

    let result = ctx
        .sql(
            "SELECT ST_Intersects(
                ST_GeomFromText('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))'),
                ST_GeomFromText('POLYGON((5 5, 6 5, 6 6, 5 6, 5 5))')
            ) as intersects",
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();

    let batch = &result[0];
    let intersects = batch
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(
        !intersects.value(0),
        "Disjoint polygons should not intersect"
    );
}

#[tokio::test]
async fn test_st_intersects_point_in_polygon() {
    let ctx = setup_ctx();

    let result = ctx
        .sql(
            "SELECT ST_Intersects(
                ST_Point(0.5, 0.5),
                ST_GeomFromText('POLYGON((0 0, 1 0, 1 1, 0 1, 0 0))')
            ) as intersects",
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();

    let batch = &result[0];
    let intersects = batch
        .column(0)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();

    assert!(intersects.value(0), "Point inside polygon should intersect");
}

#[tokio::test]
async fn test_st_intersects_with_filter() {
    let ctx = setup_ctx();

    // Create a table with polygons
    ctx.sql(
        "CREATE TABLE shapes AS VALUES
            ('a', 'POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))'),
            ('b', 'POLYGON((5 5, 7 5, 7 7, 5 7, 5 5))'),
            ('c', 'POLYGON((1 1, 3 1, 3 3, 1 3, 1 1))')",
    )
    .await
    .unwrap();

    // Find shapes that intersect with a query polygon
    let result = ctx
        .sql(
            "SELECT column1 as name FROM shapes
             WHERE ST_Intersects(
                 ST_GeomFromText(column2),
                 ST_GeomFromText('POLYGON((0 0, 2 0, 2 2, 0 2, 0 0))')
             )",
        )
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();

    // Should match 'a' (same polygon) and 'c' (overlaps)
    let total_rows: usize = result.iter().map(RecordBatch::num_rows).sum();
    assert_eq!(total_rows, 2, "Should find 2 intersecting shapes");
}
