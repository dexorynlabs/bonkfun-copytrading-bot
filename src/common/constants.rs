use anchor_lang::{prelude::Pubkey, pubkey};

pub const INIT_MSG: &str = "                                                                       
    @@@@@@   @@@@@@@@  @@@@@@@  @@@@@@@  @@@  @@@  @@@   @@@@@@@@   @@@@@@   
    @@@@@@@   @@@@@@@@  @@@@@@@  @@@@@@@  @@@  @@@@ @@@  @@@@@@@@@  @@@@@@@   
    !@@       @@!         @@!      @@!    @@!  @@!@!@@@  !@@        !@@       
    !@!       !@!         !@!      !@!    !@!  !@!!@!@!  !@!        !@!       
    !!@@!!    @!!!:!      @!!      @!!    !!@  @!@ !!@!  !@! @!@!@  !!@@!!    
    !!@!!!   !!!!!:      !!!      !!!    !!!  !@!  !!!  !!! !!@!!   !!@!!!   
        !:!  !!:         !!:      !!:    !!:  !!:  !!!  :!!   !!:       !:!  
        !:!   :!:         :!:      :!:    :!:  :!:  !:!  :!:   !::      !:!   
    :::: ::    :: ::::     ::       ::     ::   ::   ::   ::: ::::  :::: ::   
    :: : :    : :: ::      :        :     :    ::    :    :: :: :   :: : :                                                             
    ";

pub const RUN_MSG: &str = "
        @@@@@@@   @@@  @@@  @@@  @@@    @@@@@@@    @@@@@@   @@@@@@@           
        @@@@@@@@  @@@  @@@  @@@@ @@@     @@@@@@@@  @@@@@@@@  @@@@@@@           
        @@!  @@@  @@!  @@@  @@!@!@@@     @@!  @@@  @@!  @@@    @@!             
        !@!  @!@  !@!  @!@  !@!!@!@!     !@   @!@  !@!  @!@    !@!             
        @!@!!@!   @!@  !@!  @!@ !!@!     @!@!@!@   @!@  !@!    @!!             
        !!@!@!    !@!  !!!  !@!  !!!     !!!@!!!!  !@!  !!!    !!!             
        !!: :!!   !!:  !!!  !!:  !!!     !!:  !!!  !!:  !!!    !!:             
        :!:  !:!  :!:  !:!  :!:  !:!     :!:  !:!  :!:  !:!    :!:             
        ::   :::  ::::: ::   ::   ::      :: ::::  ::::: ::     ::             
        :   : :   : :  :   ::    :        :: : ::    : :  :      :                                                                                                     
       ";

pub const PUMPSWAP_FEE_ACCOUNTS: [&str; 8] = [
    "AVmoTthdrX6tKt4nDjco2D775W2YK3sDhxPcMmzUAmTY",
    "7hTckgnGnLQR6sdH7YkqFTAA7VwTfYFaZ6EhEsU3saCX",
    "62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV",
    "G5UZAVbAf46s7cKWoyKu8kYTip9DGTpbLZ2qa9Aq69dP",
    "9rPYyANsfQZw3DnDmKE3YCQF5E8oD89UXoHn9JFEhJUz",
    "JCRGumoE9Qi5BBgULTgdgTLjSgkCMSbF62ZZfGs84JeU",
    "7VtfL8fvgNfhz17qKRMjzQEXgbdpnHHHQRh54R9jP2RJ",
    "FWsW1xNtWscwNmKv6wVsU1iTzRN6wmmk3MjxRP5tT7hz"
];

pub const PUMPFUN_GLOBAL: &[u8] = b"global";
pub const PUMPFUN_PROGRAM_ADDRESS: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const BONDING_CURVE_SEED: &[u8] = b"bonding-curve";
pub const PUMPFUN_FEE_ACC: Pubkey = pubkey!("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM");
pub const PUMPFUN_FEE_ACC_DEVNET: Pubkey = pubkey!("68yFSZxzLWJXkxxRGydZ63C6mHx1NLEDWmwN9Lb5yySg");
pub const SYSTEM_PROGRAM: Pubkey = pubkey!("11111111111111111111111111111111");
pub const TOKEN_PROGRAM: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const SYSVAR_RENT_PUBKEY: Pubkey = pubkey!("SysvarRent111111111111111111111111111111111");
pub const PUMPFUN_EVENT_AUTH: Pubkey = pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
pub const ASSOCIATED_TOKEN_PROGRAM: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const TEN_THOUSAND: u64 = 10000;
pub const WSOL: &str = "So11111111111111111111111111111111111111112";
pub const PUMP_AMM_PROGRAM: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
