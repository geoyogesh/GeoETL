import type {ReactNode} from 'react';
import clsx from 'clsx';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  Svg: React.ComponentType<React.ComponentProps<'svg'>>;
  description: ReactNode;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'High Performance',
    Svg: require('@site/static/img/undraw_docusaurus_mountain.svg').default,
    description: (
      <>
        Built with Rust for blazing-fast geospatial data conversions.
        5-10x faster than traditional tools through vectorized execution
        powered by Apache DataFusion and Apache Arrow.
      </>
    ),
  },
  {
    title: 'Simple to Use',
    Svg: require('@site/static/img/undraw_docusaurus_tree.svg').default,
    description: (
      <>
        Download, extract, and start converting. No complex setup required.
        Convert between GeoJSON, CSV, and more with simple commands.
        <code>geoetl-cli convert input.geojson output.csv</code>
      </>
    ),
  },
  {
    title: '68+ Format Drivers',
    Svg: require('@site/static/img/undraw_docusaurus_react.svg').default,
    description: (
      <>
        Support for all major geospatial formats including GeoJSON, CSV,
        Shapefile, GeoPackage, and more. Built on GDAL&apos;s proven
        driver architecture with modern Rust performance.
      </>
    ),
  },
];

function Feature({title, Svg, description}: FeatureItem) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center">
        <Svg className={styles.featureSvg} role="img" />
      </div>
      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): ReactNode {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
