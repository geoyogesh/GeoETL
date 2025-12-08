---
sidebar_position: 3
title: ST_Envelope
description: Compute bounding box of a geometry
---

# ST_Envelope

Returns the minimum bounding box (envelope) of a geometry.

## Syntax

```sql
ST_Envelope(geometry)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| geometry | Geometry | Input geometry |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Bounding box as WKB polygon with `geoarrow.wkb` metadata |

## Examples

### Envelope of Polygon

```sql
SELECT ST_Envelope(ST_GeomFromText('POLYGON((0 0, 3 0, 3 2, 1 4, 0 2, 0 0))'));
-- Returns: POLYGON((0 0, 3 0, 3 4, 0 4, 0 0))
```

### Envelope of LineString

```sql
SELECT ST_Envelope(ST_GeomFromText('LINESTRING(0 0, 1 1, 2 0)'));
-- Returns: POLYGON((0 0, 2 0, 2 1, 0 1, 0 0))
```

### Calculate Extent of Features

```sql
SELECT category, ST_Envelope(ST_Union(geom)) as extent
FROM features
GROUP BY category;
```

### Pre-filter with Envelope

```sql
SELECT * FROM detailed_features
WHERE ST_Intersects(ST_Envelope(geom), search_box);
```

## Notes

- Returns the axis-aligned minimum bounding rectangle
- For Points, returns a polygon with zero area
- For horizontal/vertical lines, returns a degenerate polygon
- Useful for spatial indexing and quick intersection tests
- Envelope intersection is faster than full geometry intersection

## See Also

- [ST_ConvexHull](./st-convexhull) - Tighter bounding shape
- [ST_Centroid](./st-centroid) - Center point
- [ST_Area](../measurement/st-area) - Calculate area
