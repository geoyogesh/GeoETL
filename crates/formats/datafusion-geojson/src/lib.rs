mod file_format;
mod file_source;
mod parser;
mod physical_exec;

pub use file_format::GeoJsonFormatOptions;
pub use file_source::GeoJsonSourceBuilder;

use datafusion::prelude::*;
use datafusion_common::Result;

/// Extension trait for [`SessionContext`] that offers convenience helpers to
/// register or read `GeoJSON` sources.
#[allow(async_fn_in_trait)]
pub trait SessionContextGeoJsonExt {
    /// Register a `GeoJSON` dataset as a table with default options.
    async fn register_geojson_file(&self, name: &str, path: &str) -> Result<()>;

    /// Register a `GeoJSON` dataset with custom format options.
    async fn register_geojson_with_options(
        &self,
        name: &str,
        path: &str,
        options: GeoJsonFormatOptions,
    ) -> Result<()>;

    /// Read a `GeoJSON` dataset into a [`DataFrame`] with default options.
    async fn read_geojson_file(&self, path: &str) -> Result<DataFrame>;

    /// Read a `GeoJSON` dataset into a [`DataFrame`] with custom format options.
    async fn read_geojson_with_options(
        &self,
        path: &str,
        options: GeoJsonFormatOptions,
    ) -> Result<DataFrame>;
}

impl SessionContextGeoJsonExt for SessionContext {
    async fn register_geojson_file(&self, name: &str, path: &str) -> Result<()> {
        let options = GeoJsonFormatOptions::default();
        self.register_geojson_with_options(name, path, options)
            .await
    }

    async fn register_geojson_with_options(
        &self,
        name: &str,
        path: &str,
        options: GeoJsonFormatOptions,
    ) -> Result<()> {
        let table =
            file_source::create_geojson_table_provider(&self.state(), path, options).await?;
        self.register_table(name, table)?;
        Ok(())
    }

    async fn read_geojson_file(&self, path: &str) -> Result<DataFrame> {
        let options = GeoJsonFormatOptions::default();
        self.read_geojson_with_options(path, options).await
    }

    async fn read_geojson_with_options(
        &self,
        path: &str,
        options: GeoJsonFormatOptions,
    ) -> Result<DataFrame> {
        let table =
            file_source::create_geojson_table_provider(&self.state(), path, options).await?;
        self.read_table(table)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn register_and_query_geojson() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("points.geojson");

        let mut file = File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{
  "type": "FeatureCollection",
  "features": [
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Point",
        "coordinates": [0.0, 1.0]
      }},
      "properties": {{
        "name": "A",
        "value": 10
      }}
    }},
    {{
      "type": "Feature",
      "geometry": {{
        "type": "Point",
        "coordinates": [5.0, 2.0]
      }},
      "properties": {{
        "name": "B",
        "value": 20
      }}
    }}
  ]
}}
"#
        )
        .unwrap();

        let ctx = SessionContext::new();
        ctx.register_geojson_file("features", path.to_str().unwrap())
            .await?;

        let batches = ctx
            .sql("SELECT name, value FROM features ORDER BY value DESC")
            .await?
            .collect()
            .await?;

        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 2);

        Ok(())
    }
}
