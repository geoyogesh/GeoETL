---
sidebar_position: 4
title: ST_MakePoint
description: Create point from X, Y coordinates (alias for ST_Point)
---

# ST_MakePoint

Creates a Point geometry from X and Y coordinate values. This is an alias for [ST_Point](./st-point).

## Syntax

```sql
ST_MakePoint(x, y)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| x | Float64 | X coordinate (longitude in geographic CRS) |
| y | Float64 | Y coordinate (latitude in geographic CRS) |

## Returns

| Type | Description |
|------|-------------|
| FixedSizeList[2] | Point as GeoArrow Point with `geoarrow.point` metadata |

## Examples

### Create a Point

```sql
SELECT ST_MakePoint(-122.4194, 37.7749);
```

### Create Points from Columns

```sql
SELECT id, ST_MakePoint(lon, lat) as geom FROM gps_tracks;
```

### Use in Spatial Join

```sql
SELECT a.id, b.name
FROM points a, regions b
WHERE ST_Contains(b.geom, ST_MakePoint(a.x, a.y));
```

## Notes

- Identical to ST_Point - provided for PostGIS compatibility
- Returns GeoArrow Point format for optimal performance

## See Also

- [ST_Point](./st-point) - Primary function (same behavior)
- [ST_GeomFromText](./st-geomfromtext) - Create any geometry from WKT
