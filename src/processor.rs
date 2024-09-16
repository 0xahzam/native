use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

// Constants for better readability and maintainability
const DEPOSIT_ACCOUNT_SIZE: usize = 8;
const WITHDRAWAL_PERCENTAGE: u64 = 10;

pub fn deposit(_program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let deposit_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Check if the deposit account is already initialized
    if deposit_account.data_is_empty() {
        // If not, initialize it
        let rent = Rent::get()?;
        let rent_lamports = rent.minimum_balance(DEPOSIT_ACCOUNT_SIZE);

        invoke(
            &system_instruction::create_account(
                payer.key,
                deposit_account.key,
                rent_lamports,
                DEPOSIT_ACCOUNT_SIZE as u64,
                _program_id,
            ),
            &[
                payer.clone(),
                deposit_account.clone(),
                system_program.clone(),
            ],
        )?;
    }

    // Transfer the deposit amount
    invoke(
        &system_instruction::transfer(payer.key, deposit_account.key, amount),
        &[
            payer.clone(),
            deposit_account.clone(),
            system_program.clone(),
        ],
    )?;

    // Update the total deposited amount
    let mut deposit_data = deposit_account.try_borrow_mut_data()?;
    let mut total_deposited = u64::from_le_bytes(deposit_data[..8].try_into().unwrap());
    total_deposited += amount;
    deposit_data[..8].copy_from_slice(&total_deposited.to_le_bytes());

    Ok(())
}

pub fn withdraw(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let deposit_account = next_account_info(accounts_iter)?;
    let recipient = next_account_info(accounts_iter)?;

    // Ensure the deposit account is owned by the program
    if deposit_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut deposit_data = deposit_account.try_borrow_mut_data()?;
    let mut total_deposited = u64::from_le_bytes(deposit_data[..8].try_into().unwrap());
    let withdrawal_amount = total_deposited / WITHDRAWAL_PERCENTAGE;

    if withdrawal_amount == 0 {
        return Err(ProgramError::InsufficientFunds);
    }

    // Calculate the withdrawal amount
    let withdrawal_amount = std::cmp::min(withdrawal_amount, **deposit_account.lamports.borrow());

    // Transfer the withdrawal amount
    **deposit_account.try_borrow_mut_lamports()? -= withdrawal_amount;
    **recipient.try_borrow_mut_lamports()? += withdrawal_amount;

    // Update the total deposited amount
    total_deposited -= withdrawal_amount;
    deposit_data[..8].copy_from_slice(&total_deposited.to_le_bytes());

    Ok(())
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum TransferInstruction {
    DepositInstruction(u64),
    WithdrawalInstruction,
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = TransferInstruction::try_from_slice(input)?;
    match instruction {
        TransferInstruction::DepositInstruction(amount) => deposit(program_id, accounts, amount),
        TransferInstruction::WithdrawalInstruction => withdraw(program_id, accounts),
    }
}
