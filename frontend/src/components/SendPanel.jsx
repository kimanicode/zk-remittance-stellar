import React, { useState, useCallback } from "react";
import ProofStatus from "./ProofStatus.jsx";
import { generateProof } from "../lib/proof.js";
import { submitProof } from "../lib/stellar.js";

const styles = {
  panel: {
    padding: "1.5rem",
    borderRadius: 12,
    border: "1px solid #e9ecef",
    background: "#fff",
  },
  title: {
    fontSize: "1.1rem",
    fontWeight: 600,
    marginTop: 0,
    marginBottom: "1rem",
  },
  field: {
    marginBottom: "0.75rem",
  },
  label: {
    display: "block",
    fontSize: "0.8rem",
    fontWeight: 500,
    color: "#495057",
    marginBottom: "0.25rem",
  },
  input: {
    width: "100%",
    padding: "0.5rem 0.75rem",
    borderRadius: 6,
    border: "1px solid #ced4da",
    fontSize: "0.9rem",
    boxSizing: "border-box",
  },
  button: {
    width: "100%",
    padding: "0.6rem",
    borderRadius: 6,
    border: "none",
    background: "#4263f5",
    color: "#fff",
    fontSize: "0.95rem",
    fontWeight: 600,
    cursor: "pointer",
    marginTop: "0.5rem",
  },
  buttonDisabled: {
    opacity: 0.6,
    cursor: "not-allowed",
  },
  error: {
    color: "#c92a2a",
    fontSize: "0.85rem",
    marginTop: "0.5rem",
  },
};

export default function SendPanel() {
  const [recipient, setRecipient] = useState("");
  const [amount, setAmount] = useState("");
  const [steps, setSteps] = useState(["pending", "pending", "pending"]);
  const [txHash, setTxHash] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSend = useCallback(async () => {
    setError("");
    setTxHash("");
    setSteps(["active", "pending", "pending"]);
    setLoading(true);

    try {
      const amountNum = parseFloat(amount);
      if (isNaN(amountNum) || amountNum <= 0) throw new Error("Invalid amount");
      if (amountNum >= 10000) throw new Error("Amount must be under $10,000");

      const senderAddress =
        "GBX3XPJR4NIO2IVDJNVFHMIBWFVQXBN7IWFMY5FK6IALSOFGAO6BVKNX";

      // Step 1: Generate proof
      const {
        proofAHex,
        proofBHex,
        proofCHex,
        publicSignalsDecimal,
        merkleRootHex,
        nullifierHashHex,
        recipientHashHex,
      } = await generateProof({
        senderAddress,
        recipientAddress: recipient,
        amount: amountNum,
        merkleRoot:
          "40358931300632933350769883338497611674297341631427651364528416116208948793141",
      });
      setSteps(["done", "active", "pending"]);

      // Step 2: Submit to Stellar
      const result = await submitProof({
        senderKeypair: import.meta.env.VITE_ALICE_SECRET, // TODO: integrate wallet
        proofAHex,
        proofBHex,
        proofCHex,
        publicSignalsDecimal,
        merkleRootHex,
        nullifierHashHex,
        recipientHashHex,
        recipient,
        amount: Math.round(amountNum * 1e7),
      });
      setSteps(["done", "done", "pending"]);

      // Step 3: Wait for confirmation
      if (result.hash) {
        setTxHash(result.hash);
        setSteps(["done", "done", "done"]);
      } else if (result.status === "PENDING") {
        setTxHash(`Pending... check ${result.hash}`);
        setSteps(["done", "done", "pending"]);
      } else {
        throw new Error(result.error || "Transaction failed");
      }
    } catch (err) {
      setError(err.message || "An error occurred");
      setSteps((prev) => prev.map((s, i) => (s === "active" ? "error" : s)));
    } finally {
      setLoading(false);
    }
  }, [recipient, amount]);

  return (
    <div style={styles.panel}>
      <h2 style={styles.title}>Send Payment</h2>

      <div style={styles.field}>
        <label style={styles.label}>Recipient Stellar Address</label>
        <input
          style={styles.input}
          placeholder="G..."
          value={recipient}
          onChange={(e) => setRecipient(e.target.value)}
        />
      </div>

      <div style={styles.field}>
        <label style={styles.label}>Amount (USD, max $9,999)</label>
        <input
          style={styles.input}
          type="number"
          min="0.01"
          max="9999"
          step="0.01"
          placeholder="5000"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
        />
      </div>

      <button
        style={{ ...styles.button, ...(loading ? styles.buttonDisabled : {}) }}
        onClick={handleSend}
        disabled={loading || !recipient || !amount}
      >
        {loading ? "Processing..." : "Generate Proof & Send"}
      </button>

      {error && <div style={styles.error}>{error}</div>}

      <ProofStatus steps={steps} txHash={txHash} />
    </div>
  );
}
