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
    Svg: require('@site/static/img/icon-performance.svg').default,
    description: (
      <>
        Built with Rust for blazing-fast geospatial data conversions.
        Leverages vectorized execution powered by Apache DataFusion
        and Apache Arrow for optimal throughput.
      </>
    ),
  },
  {
    title: 'Simple to Use',
    Svg: require('@site/static/img/icon-simple.svg').default,
    description: (
      <>
        Download, extract, and start converting. No complex setup required.
        Convert between GeoJSON, CSV, and more with simple commands.
        <code>geoetl-cli convert input.geojson output.csv</code>
      </>
    ),
  },
  {
    title: 'Multiple Format Support',
    Svg: require('@site/static/img/icon-formats.svg').default,
    description: (
      <>
        Support for major geospatial formats including GeoJSON, GeoParquet,
        CSV with WKT geometries, and more. Built on proven standards with
        modern Rust performance.
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
