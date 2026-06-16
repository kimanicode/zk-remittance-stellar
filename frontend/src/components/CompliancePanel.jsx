import React, { useState, useCallback } from 'react';
import { queryCompliance } from '../lib/stellar.js';

const styles = {
  panel: {
    padding: '1.5rem',
    borderRadius: 12,
    border: '1px solid #e9ecef',
    background: '#fff',
  },
  title: {
    fontSize: '1.1rem',
    fontWeight: 600,
    marginTop: 0,
    marginBottom: '1rem',
  },
  field: {
    marginBottom: '0.75rem',
  },
  label: {
    display: 'block',
    fontSize: '0.8rem',
    fontWeight: 500,
    color: '#495057',
    marginBottom: '0.25rem',
  },
  input: {
    width: '100%',
    padding: '0.5rem 0.75rem',
    borderRadius: 6,
    border: '1px solid #ced4da',
    fontSize: '0.9rem',
    boxSizing: 'border-box',
    fontFamily: 'monospace',
  },
  button: {
    width: '100%',
    padding: '0.6rem',
    borderRadius: 6,
    border: 'none',
    background: '#2b8a3e',
    color: '#fff',
    fontSize: '0.95rem',
    fontWeight: 600,
    cursor: 'pointer',
    marginTop: '0.5rem',
  },
  buttonDisabled: {
    opacity: 0.6,
    cursor: 'not-allowed',
  },
  result: {
    marginTop: '1rem',
    padding: '1rem',
    borderRadius: 8,
    fontSize: '0.9rem',
  },
  resultUnder: {
    background: '#d3f9d8',
    border: '1px solid #b2f2bb',
    color: '#2b8a3e',
  },
  resultOver: {
    background: '#ffe3e3',
    border: '1px solid #ffc9c9',
    color: '#c92a2a',
  },
  nullifierLabel: {
    fontSize: '0.7rem',
    color: '#868e96',
    marginTop: '0.5rem',
  },
  nullifier: {
    fontFamily: 'monospace',
    fontSize: '0.7rem',
    wordBreak: 'break-all',
    color: '#495057',
  },
  error: {
    color: '#c92a2a',
    fontSize: '0.85rem',
    marginTop: '0.5rem',
  },
};

export default function CompliancePanel() {
  const [addressHash, setAddressHash] = useState('');
  const [threshold, setThreshold] = useState('10000');
  const [result, setResult] = useState(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleQuery = useCallback(async () => {
    setError('');
    setResult(null);
    setLoading(true);

    try {
      const res = await queryCompliance({
        addressHash: addressHash.startsWith('0x') ? addressHash.slice(2) : addressHash,
        threshold: parseFloat(threshold) * 100, // USD to cents
      });
      setResult(res);
    } catch (err) {
      setError(err.message || 'Query failed');
    } finally {
      setLoading(false);
    }
  }, [addressHash, threshold]);

  return (
    <div style={styles.panel}>
      <h2 style={styles.title}>Compliance Query</h2>
      <p style={{ fontSize: '0.8rem', color: '#868e96', marginTop: '-0.5rem', marginBottom: '1rem' }}>
        Check if an address hash has exceeded the threshold. No identity is revealed.
      </p>

      <div style={styles.field}>
        <label style={styles.label}>Address Hash (hex)</label>
        <input
          style={styles.input}
          placeholder="0x..."
          value={addressHash}
          onChange={e => setAddressHash(e.target.value)}
        />
      </div>

      <div style={styles.field}>
        <label style={styles.label}>Threshold (USD)</label>
        <input
          style={styles.input}
          type="number"
          min="0"
          step="100"
          value={threshold}
          onChange={e => setThreshold(e.target.value)}
        />
      </div>

      <button
        style={{ ...styles.button, ...(loading ? styles.buttonDisabled : {}) }}
        onClick={handleQuery}
        disabled={loading || !addressHash}
      >
        {loading ? 'Querying...' : 'Query Compliance'}
      </button>

      {error && <div style={styles.error}>{error}</div>}

      {result && (
        <div
          style={{
            ...styles.result,
            ...(result.exceeded ? styles.resultOver : styles.resultUnder),
          }}
        >
          <strong>{result.exceeded ? 'Threshold Exceeded' : 'Under Threshold'}</strong>
          <div style={styles.nullifierLabel}>Query Proof (Nullifier Hash):</div>
          <div style={styles.nullifier}>{result.nullifierHash}</div>
        </div>
      )}
    </div>
  );
}
