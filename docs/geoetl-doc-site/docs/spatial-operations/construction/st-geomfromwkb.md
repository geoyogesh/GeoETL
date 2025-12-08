---
sidebar_position: 2
title: ST_GeomFromWKB
description: Validate WKB binary data as geometry
---

# ST_GeomFromWKB

Validates Well-Known Binary (WKB) data and returns it as a geometry.

## Syntax

```sql
ST_GeomFromWKB(wkb)
```

## Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| wkb | Binary | Well-Known Binary representation of a geometry |

## Returns

| Type | Description |
|------|-------------|
| Binary (WKB) | Validated geometry as WKB with `geoarrow.wkb` metadata |

## Examples

### Validate WKB Data

```sql
SELECT ST_GeomFromWKB(wkb_column) as geom FROM spatial_data;
```

### Use with Chained Operations

```sql
SELECT ST_Area(ST_GeomFromWKB(wkb_data)) as area FROM parcels;
```

### Filter Invalid Geometries

```sql
SELECT * FROM raw_data
WHERE ST_GeomFromWKB(wkb_column) IS NOT NULL;
```

## Notes

- Validates that the binary data is valid WKB format
- Adds `geoarrow.wkb` metadata to the output
- Invalid WKB will result in an error
- Useful when importing data from external sources that may contain malformed geometries

## See Also

- [ST_GeomFromText](./st-geomfromtext) - Create geometry from WKT
- [ST_IsValid](../validator/st-isvalid) - Test geometry validity
