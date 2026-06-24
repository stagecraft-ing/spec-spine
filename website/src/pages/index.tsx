import React from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import styles from './index.module.css';

function HomepageHero() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--primary', styles.heroBanner)}>
      <div className="container">
        <h1 className="hero__title">{siteConfig.title}</h1>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <div className={styles.installBlock}>
          <code>cargo install spec-spine-cli --version 0.8.0 --locked</code>
        </div>
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg"
            to="/docs/getting-started/installation">
            Get Started
          </Link>
          <Link
            className="button button--outline button--secondary button--lg"
            to="/docs/cli/overview"
            style={{marginLeft: '1rem'}}>
            CLI Reference
          </Link>
        </div>
      </div>
    </header>
  );
}

type FeatureItem = {
  title: string;
  description: string;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'Typed authority graph',
    description:
      'Eight typed edges and six unit kinds form a directed authority graph. Ownership is derived by walking the graph, not declared directly.',
  },
  {
    title: 'Deterministic compiler',
    description:
      'Every artifact-producing function is pure: same inputs produce byte-identical output on every platform. Content hashes over normalized POSIX paths.',
  },
  {
    title: 'PR-time coupling gate',
    description:
      'The coupling gate joins the spec-as-source registry and the code-as-source index at PR time and refuses drift. Exit 1 on uncovered paths.',
  },
  {
    title: 'Adopt in any repo',
    description:
      'Four install channels (cargo, curl, npm, pip), one config file, zero source edits to the library. Every project-specific assumption is a knob.',
  },
];

function Feature({title, description}: FeatureItem) {
  return (
    <div className={clsx('col col--3')}>
      <div className="padding-horiz--md padding-vert--lg">
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
    </div>
  );
}

function AdoptionKitCard() {
  return (
    <div className={styles.adoptionCard}>
      <div className="container">
        <div className="row">
          <div className="col col--8 col--offset-2">
            <div className={styles.adoptionCardInner}>
              <h2>Adopt the governed workflow</h2>
              <p>
                spec-spine + the Claude Code skill kit: a complete governed
                development workflow for agentic software engineering.
              </p>
              <Link
                className="button button--primary button--lg"
                to="/docs/claude-code/overview">
                View the Adoption Kit
              </Link>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function Home(): React.JSX.Element {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title={siteConfig.title}
      description="A typed, hash-verifiable authority ledger over a markdown spec corpus">
      <HomepageHero />
      <main>
        <section className={styles.features}>
          <div className="container">
            <div className="row">
              {FeatureList.map((props, idx) => (
                <Feature key={idx} {...props} />
              ))}
            </div>
          </div>
        </section>
        <AdoptionKitCard />
      </main>
    </Layout>
  );
}
