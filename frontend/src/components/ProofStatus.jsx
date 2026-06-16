import React from 'react';

const styles = {
  container: {
    marginTop: '1rem',
    padding: '0.75rem',
    borderRadius: 8,
    background: '#f8f9fa',
    border: '1px solid #e9ecef',
  },
  step: {
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
    padding: '0.25rem 0',
    fontSize: '0.85rem',
  },
  spinner: {
    width: 14,
    height: 14,
    border: '2px solid #dee2e6',
    borderTopColor: '#4263f5',
    borderRadius: '50%',
    animation: 'spin 0.6s linear infinite',
  },
  check: {
    color: '#2b8a3e',
    fontWeight: 700,
  },
  cross: {
    color: '#c92a2a',
    fontWeight: 700,
  },
  hash: {
    fontFamily: 'monospace',
    fontSize: '0.75rem',
    color: '#495057',
    wordBreak: 'break-all',
  },
};

const STEP_LABELS = [
  'Generating zero-knowledge proof...',
  'Submitting to Stellar...',
  'Confirmed on-chain',
];

export default function ProofStatus({ steps, txHash }) {
  return (
    <div style={styles.container}>
      {STEP_LABELS.map((label, i) => {
        const status = steps[i];
        let icon;
        if (status === 'done') icon = <span style={styles.check}>&#10003;</span>;
        else if (status === 'error') icon = <span style={styles.cross}>&#10007;</span>;
        else if (status === 'active') icon = <div style={styles.spinner} />;
        else icon = <span style={{ color: '#adb5bd' }}>&#9679;</span>;

        return (
          <div key={i} style={styles.step}>
            {icon}
            <span style={{ color: status === 'done' ? '#2b8a3e' : status === 'active' ? '#4263f5' : '#868e96' }}>
              {label}
            </span>
          </div>
        );
      })}
      {txHash && (
        <div style={{ marginTop: '0.5rem', paddingTop: '0.5rem', borderTop: '1px solid #dee2e6' }}>
          <div style={styles.hash}>Tx: {txHash}</div>
        </div>
      )}
    </div>
  );
}
