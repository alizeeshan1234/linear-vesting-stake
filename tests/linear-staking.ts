import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LinearStaking } from "../target/types/linear_staking";
import { createMint, getAssociatedTokenAddress, getOrCreateAssociatedTokenAccount, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, mintTo, createAssociatedTokenAccount } from "@solana/spl-token";
import { PublicKey, SystemProgram } from "@solana/web3.js";

// Helper function to sleep
const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

describe("linear-staking", () => {
  let provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.linearStaking as Program<LinearStaking>;

  let tokenMint: PublicKey;
  let stakeVaultPDA: PublicKey;
  let vaultTokenAccountPDA: PublicKey;
  let transferAuthorityPDA: PublicKey;
  let userTokenAccountPDA: PublicKey;
  let userStakeAccountPDA: PublicKey;

  // Use a longer vesting period for testing partial claims (10 seconds)
  const VESTING_PERIOD_SECONDS = 10;

  before(async () => {
    // Create a new mint for testing
    tokenMint = await createMint(
      provider.connection,
      provider.wallet.payer,
      provider.wallet.publicKey,
      null,
      0,
    );

    let userTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      tokenMint,
      provider.wallet.publicKey
    );

    userTokenAccountPDA = userTokenAccount.address;

    // Derive the stake vault PDA
    [stakeVaultPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("stake_vault")],
      program.programId
    );

    // Derive the vault token account PDA
    [vaultTokenAccountPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("stake_vault_token_account")],
      program.programId
    );

    [transferAuthorityPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("transfer_authority")],
      program.programId
    );

    [userStakeAccountPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_stake"), provider.wallet.publicKey.toBuffer()],
      program.programId
    );
  });

  it("Initialize stake vault", async () => {

    let initialize_vesting_params = {
      vestingPeriod: new anchor.BN(VESTING_PERIOD_SECONDS)
    }

    const tx = await program.methods.initialize(initialize_vesting_params).accountsPartial({
      admin: provider.wallet.publicKey,
      tokenMint: tokenMint,
      stakeVault: stakeVaultPDA,
      vaultTokenAccount: vaultTokenAccountPDA,
      transferAuthority: transferAuthorityPDA,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID
    }).signers([provider.wallet.payer]).rpc();

    console.log("Initialization transaction signature:", tx);
  });

  it("Deposit Stake", async () => {

    await mintTo(
      provider.connection,
      provider.wallet.payer,
      tokenMint,
      userTokenAccountPDA,
      provider.wallet.publicKey,
      2000
    );

    let depositAmount = new anchor.BN(1000);

    let depositStakeParams = {
      amount: depositAmount
    }

    const tx = await program.methods.depositStake(depositStakeParams).accountsPartial({
      owner: provider.wallet.publicKey,
      feePayer: provider.wallet.publicKey,
      userTokenAccount: userTokenAccountPDA,
      stakeVault: stakeVaultPDA,
      vaultTokenAccount: vaultTokenAccountPDA,
      userStake: userStakeAccountPDA,
      tokenMint: tokenMint,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([provider.wallet.payer]).rpc();

    console.log("Deposit Stake transaction signature:", tx);
  });

  it("Request Unstake", async () => {
    let unstakeRequestParams = {
      amount: new anchor.BN(1000)
    };

    const tx = await program.methods.unstakeRequest(unstakeRequestParams).accountsPartial({
      owner: provider.wallet.publicKey,
      stakeVault: stakeVaultPDA,
      userStake: userStakeAccountPDA,
    }).signers([provider.wallet.payer]).rpc();

    console.log("Unstake Request transaction signature:", tx);
  });

  it("Claim partial vested tokens (50%)", async () => {
    // Wait for HALF the vesting period to test partial claiming
    const halfPeriod = Math.floor(VESTING_PERIOD_SECONDS / 2);
    console.log(`Waiting ${halfPeriod} seconds (50% of vesting period)...`);
    await sleep(halfPeriod * 1000);

    let claimVestedParams = {
      requestId: null
    };

    const tx = await program.methods.claimVested(claimVestedParams).accountsPartial({
      owner: provider.wallet.publicKey,
      userTokenAccount: userTokenAccountPDA,
      stakeVault: stakeVaultPDA,
      vaultTokenAccount: vaultTokenAccountPDA,
      transferAuthority: transferAuthorityPDA,
      userStake: userStakeAccountPDA,
      tokenMint: tokenMint,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([provider.wallet.payer]).rpc();

    console.log("Partial claim transaction signature:", tx);
    // Should claim ~500 tokens (50% of 1000)
  });

  it("Claim remaining vested tokens", async () => {
    // Wait for the rest of the vesting period
    const remainingTime = Math.ceil(VESTING_PERIOD_SECONDS / 2) + 1;
    console.log(`Waiting ${remainingTime} seconds for remaining vesting...`);
    await sleep(remainingTime * 1000);

    let claimVestedParams = {
      requestId: null
    };

    const tx = await program.methods.claimVested(claimVestedParams).accountsPartial({
      owner: provider.wallet.publicKey,
      userTokenAccount: userTokenAccountPDA,
      stakeVault: stakeVaultPDA,
      vaultTokenAccount: vaultTokenAccountPDA,
      transferAuthority: transferAuthorityPDA,
      userStake: userStakeAccountPDA,
      tokenMint: tokenMint,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([provider.wallet.payer]).rpc();

    console.log("Final claim transaction signature:", tx);
    // Should claim remaining ~500 tokens
  })
});
