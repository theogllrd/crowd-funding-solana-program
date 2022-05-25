// First we include what we are going to need in our program.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

// Every solana program has one entry point
// It should take in program_id, accounts, instruction_data as parameters.
fn process_instruction(
    // program id is the id of this program on the solana network.
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    // This is the data we want to process our instruction for, it is a list of 8 bitunsigned integers(0..255).
    instruction_data: &[u8],
) -> ProgramResult {
    if instruction_data.len() == 0 {
        return Err(ProgramError::InvalidInstructionData);
    }

    if instruction_data[0] == 0 {
        return create_campaign(
            program_id,
            accounts,
            // we pass a reference to slice of [instruction_data], we do not want the first element in any of our functions.
            &instruction_data[1..instruction_data.len()],
        );
    } else if instruction_data[0] == 1 {
        return withdraw(
            program_id,
            accounts,
            &instruction_data[1..instruction_data.len()],
        );
    } else if instruction_data[0] == 2 {
        return donate(
            program_id,
            accounts,
            &instruction_data[1..instruction_data.len()],
        );
    }

    msg!("Didn't find the entrypoint required");
    Err(ProgramError::InvalidInstructionData)
}

// Then we call the entry point macro to add `process_instruction` as our entry point to our program.
entrypoint!(process_instruction);

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct CampaignDetails {
    pub admin: Pubkey,
    pub name: String,
    pub description: String,
    pub image_link: String,
    pub amount_donated: u64,
}

fn create_campaign(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    // Writing account is the account we're gonna write on it
    // This is an account we will create in our front-end.
    // This account should br owned by the solana program.
    let writing_account = next_account_info(accounts_iter)?;

    // Account of the person creating the campaign.
    let creator_account = next_account_info(accounts_iter)?;

    // Now to allow transactions we want the creator account to sign the transaction.instruction_data
    if !creator_account.is_signer {
        msg!("creator_account must be a signer");
        return Err(ProgramError::IncorrectProgramId);
    }
    // We want to write in this account so we want its owner by the program.
    if writing_account.owner != program_id {
        msg!("writing_account is'nt owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut input_data = CampaignDetails::try_from_slice(&instruction_data)
        .expect("Instruction data serialization didn't worked");

    // Now I want that for a campaign created the only admin should be the one who created it.
    if input_data.admin != *creator_account.key {
        msg!("Invalid instruction data, admin isn't the creator");
        return Err(ProgramError::InvalidInstructionData);
    }

    // get the minimum balance we need in our program account
    let rent_exemption = Rent::get()?.minimum_balance(writing_account.data_len());

    // and we make sure our wrinting_account has that much lamports(balance)
    if **writing_account.lamports.borrow() < rent_exemption {
        msg!("The balance of writing_account must be more then rent_exemption");
        return Err(ProgramError::InsufficientFunds);
    }

    // Then we can set the initial amount donate to be zero.
    input_data.amount_donated = 0;

    // If everything went well, we write all the data into the writing_account
    input_data.serialize(&mut &mut writing_account.data.borrow_mut()[..])?;

    Ok(())
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct WithdrawRequest {
    pub amount: u64,
}

fn withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // create a new iteration on accounts
    let accounts_iter = &mut accounts.iter();
    let writing_account = next_account_info(accounts_iter)?;
    let admin_account = next_account_info(accounts_iter)?;

    // We check if the writing account is owned by program.
    if writing_account.owner != program_id {
        msg!("writing_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Admin account should be the signer in this transaction.
    if !admin_account.is_signer {
        msg!("admin should be signer");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut campaign_data = CampaignDetails::try_from_slice(*writing_account.data.borrow())
        .expect("Error deserializing data");

    // Then we check if the admin_account's public key is equal to
    // the public key we have stored in our campaign_data.
    if campaign_data.admin != *admin_account.key {
        msg!("Only the account admin can withdraw");
        return Err(ProgramError::InvalidAccountData);
    }

    // Here we make use of the struct we created.
    // We will get the amount of lamports admin wants to withdraw
    let input_data = WithdrawRequest::try_from_slice(&instruction_data)
        .expect("Instruction data serialization didn't worked");

    // we don't want the campaign to be deleted after a withdrawal, so we check the rent-exempt
    let rent_exemption = Rent::get()?.minimum_balance(writing_account.data_len());

    // We check if we have enough funds
    if **writing_account.lamports.borrow() - rent_exemption < input_data.amount {
        msg!("Insufficent balance");
        return Err(ProgramError::InsufficientFunds);
    }

    // I everything went well, we transfere balance
    **writing_account.try_borrow_mut_lamports()? -= input_data.amount;
    **admin_account.try_borrow_mut_lamports()? += input_data.amount;
    Ok(())
}

fn donate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let writing_account = next_account_info(accounts_iter)?;
    // this account would be create in the front-end, and only has the Lamport we would like to donate
    let donator_program_account = next_account_info(accounts_iter)?;
    let donator = next_account_info(accounts_iter)?;

    if writing_account.owner != program_id {
        msg!("writing_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if donator_program_account.owner != program_id {
        msg!("donator_program_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if !donator.is_signer {
        msg!("donator should be signer");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut campaign_data = CampaignDetails::try_from_slice(*writing_account.data.borrow())
        .expect("Error deserializing data");

    // we increase the total amount donated by the amount in our donator program account
    campaign_data.amount_donated += **donator_program_account.lamports.borrow();

    // we do the actual transaction
    **writing_account.try_borrow_mut_lamports()? += **donator_program_account.lamports.borrow();
    **donator_program_account.try_borrow_mut_lamports()? = 0;

    // we will write the new updated campaign_data to the writing_account
    campaign_data.serialize(&mut &mut writing_account.data.borrow_mut()[..])?;

    Ok(())
}
