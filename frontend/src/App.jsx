import React from 'react';
import SendPanel from './components/SendPanel.jsx';
import CompliancePanel from './components/CompliancePanel.jsx';

const styles = {
  container: {
    maxWidth: 1200,
    margin: '0 auto',
    padding: '2rem 1rem',
    fontFamily: 'system-ui, -apple-system, sans-serif',
  },
  header: {
    textAlign: 'center',
    marginBottom: '2rem',
  },
  title: {
    fontSize: '1.75rem',
    fontWeight: 700,
    margin: 0,
  },
  subtitle: {
    fontSize: '0.9rem',
    color: '#666',
    marginTop: '0.25rem',
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: '1fr 1fr',
    gap: '1.5rem',
  },
  footer: {
    marginTop: '3rem',
    textAlign: 'center',
    fontSize: '0.75rem',
    color: '#999',
  },
};

export default function App() {
  return (
    <div style={styles.container}>
      <header style={styles.header}>
        <h1 style={styles.title}>ZK Remittance</h1>
        <p style={styles.subtitle}>
          Private cross-border payments on Stellar with zero-knowledge proofs
        </p>
      </header>

      <div style={styles.grid}>
        <SendPanel />
        <CompliancePanel />
      </div>

      <footer style={styles.footer}>
        Stellar Testnet · Groth16 · Poseidon Hash · Soroban
      </footer>
    </div>
  );
}
