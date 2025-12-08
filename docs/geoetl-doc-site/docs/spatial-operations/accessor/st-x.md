---
sidebar_position: 1
title: ST_X
description: Get X coordinate of a Point
---

# ST_X

Returns the X coordinate of a Point geometry.

## Syntax

```sql
ST_X(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Point geometry |

## Returns

| Type | Description |
|------|-------------|
| Float64 | X coordinate value |

## Examples

### Get X Coordinate

```sql
SELECT ST_X(ST_Point(10.5, 20.3));
-- Returns: 10.5
```

### Extract Coordinates from Points

```sql
SELECT id, ST_X(geom) as longitude, ST_Y(geom) as latitude
FROM locations;
```

### Filter by X Range

```sql
SELECT * FROM points
WHERE ST_X(geom) BETWEEN -122.5 AND -122.0;
```

### Use with Centroid

```sql
SELECT id, ST_X(ST_Centroid(geom)) as center_x
FROM polygons;
```

## Notes

- Only valid for Point geometries
- For geographic data, X typically represents longitude
- Returns NULL if geometry is not a Point
- Works with both GeoArrow Point and WKB Point formats

## See Also

- [ST_Y](./st-y) - Get Y coordinate
- [ST_Point](../construction/st-point) - Create point from coordinates
- [ST_Centroid](../generator/st-centroid) - Get center point
