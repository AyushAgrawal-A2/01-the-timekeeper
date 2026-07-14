// Timekeeper (Solana Summit India challenge c1) solver
// Usage:  node solve.js /path/to/keypair.json
// Defaults to the Solana CLI keypair if no path is given.
const {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  sendAndConfirmTransaction,
} = require("@solana/web3.js");
const crypto = require("crypto");
const fs = require("fs");
const os = require("os");

const PROGRAM_ID = new PublicKey(
  "CEjNzYQz8ytqh2rG5azXeqBiA7TfPYWjJxYXh8ApaC9c",
);
const RPC = process.env.RPC || "https://api.devnet.solana.com";

// The "time it really keeps": genesis seed carried forward 64 chimes.
// M = sha256^64(config[8:40]); this is a fixed on-chain constant (verified).
const M = Buffer.from(
  "a4222455658fab566fadb77ba40af147131286b8599d2734ad91434fa8c1e1c8",
  "hex",
);

function sha256(...b) {
  const h = crypto.createHash("sha256");
  for (const x of b) h.update(x);
  return h.digest();
}

async function main() {
  const kpPath = process.argv[2] || `${os.homedir()}/.config/solana/id.json`;
  const secret = Uint8Array.from(JSON.parse(fs.readFileSync(kpPath, "utf8")));
  const me = Keypair.fromSecretKey(secret);
  const conn = new Connection(RPC, "confirmed");
  console.log("Wallet :", me.publicKey.toBase58());

  const [record] = PublicKey.findProgramAddressSync(
    [Buffer.from("progress"), me.publicKey.toBuffer()],
    PROGRAM_ID,
  );
  const [config] = PublicKey.findProgramAddressSync(
    [Buffer.from("oracle")],
    PROGRAM_ID,
  );
  console.log("Record :", record.toBase58());
  console.log("Config :", config.toBase58());

  const bal = await conn.getBalance(me.publicKey);
  console.log("Balance:", bal / 1e9, "SOL");
  if (bal < 2e6)
    throw new Error(
      "Fund this wallet on devnet first: solana airdrop 1 -u devnet",
    );

  // ---- WAKE (idempotent) ----
  let rec = await conn.getAccountInfo(record);
  if (!rec) {
    console.log("\n[wake] recording your moment ...");
    const wake = new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [
        { pubkey: me.publicKey, isSigner: true, isWritable: true },
        { pubkey: record, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data: Buffer.from([0x01]),
    });
    const sig = await sendAndConfirmTransaction(
      conn,
      new Transaction().add(wake),
      [me],
    );
    console.log("[wake] tx:", sig);
    rec = await conn.getAccountInfo(record);
  } else {
    console.log("\n[wake] already awake — reusing existing record.");
  }

  // record layout: "TMKPR1"(6) tag(1) wallet(32) arrival_slot(u64 LE @39) n(u32 @47) solved(u8 @51) ...
  const data = rec.data;
  if (data[51] === 1) {
    console.log("\nAlready cleared. The Timekeeper remembers you. Done.");
    return;
  }
  const arrival = data.readBigUInt64LE(39);
  console.log("[wake] arrival slot:", arrival.toString());

  // ---- PROOF ----
  const arr8 = Buffer.alloc(8);
  arr8.writeBigUInt64LE(arrival);
  const proof = sha256(me.publicKey.toBuffer(), M, arr8);
  console.log("[proof]:", proof.toString("hex"));

  // ---- CLEAR ----
  const clear = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: me.publicKey, isSigner: true, isWritable: true },
      { pubkey: record, isSigner: false, isWritable: true },
      { pubkey: config, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([Buffer.from([0x02]), proof]),
  });
  const tx = new Transaction().add(clear);
  tx.feePayer = me.publicKey;
  tx.recentBlockhash = (await conn.getLatestBlockhash()).blockhash;
  const sim = await conn.simulateTransaction(tx);
  console.log("\n[clear] simulation err:", sim.value.err);
  if (sim.value.err) {
    console.log("logs:", sim.value.logs);
    throw new Error("simulation failed — not sending");
  }
  const sig = await sendAndConfirmTransaction(
    conn,
    new Transaction().add(clear),
    [me],
  );
  console.log("[clear] tx:", sig);

  const after = await conn.getAccountInfo(record);
  console.log(
    "\nsolved flag:",
    after.data[51],
    after.data[51] === 1
      ? "✓ CLEARED — remembered on chain, for good."
      : "(unexpected)",
  );
}
main().catch((e) => {
  console.error("ERROR:", e.message);
  process.exit(1);
});
