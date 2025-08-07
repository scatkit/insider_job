import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { InsidersJob } from "../target/types/insiders_job";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import dotenv from "dotenv";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

describe("insiders_job", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  dotenv.config();

  const program = anchor.workspace.insidersJob as Program<InsidersJob>;
  const adminKey = anchor.getProvider().wallet.payer;
  console.log("Admin key:", adminKey.publicKey.toString())
  const secret = process.env.SECRET!;
  console.log("SECRET:", process.env.SECRET); // should print the string
  const fakeWallet = Keypair.fromSecretKey(bs58.decode(secret));
  console.log("Fake wallet", fakeWallet.publicKey.toString())
  const [configPDA, bump] = PublicKey.findProgramAddressSync(
    [Buffer.from("config"), program.programId.toBuffer()],
    program.programId,
  )

  it("Initialize Config", async () => {
    const feeRate = new anchor.BN(300); // 3% fee
    const minStake = new anchor.BN(40000000) // 40M lamports
    const tx = await program.methods.initializeConfig(feeRate, minStake)
      .accounts({
        admin: adminKey.publicKey,
        config: configPDA,
        systemProgram: SystemProgram.programId,
      }
      )
      .signers([adminKey])
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it("Update Config", async () => {
    const newFeeRate = new anchor.BN(400); // 3% fee
    const newMinStake = new anchor.BN(60000000) // 40M lamports
    const tx = await program.methods.updateConfig(newFeeRate, newMinStake)
      .accounts({
        admin: adminKey.publicKey,
        config: configPDA,
      })
      .signers([adminKey])
      .rpc()
    console.log("Your transaction signature", tx);

  })

  it("Initialize Market" async () => {
    const tokenAddress = new PublicKey("8Ga7ExC7toM9v1PqCB8jjKRTKqiYqQGLCua2VnXdbonk");
    const marketMint = Keypair.generate();
    // const tx = await program.methods.initializeMarket()
  })

});
