use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token_interface::{
    TokenAccount, Mint, TokenInterface, TransferChecked, transfer_checked,
};

declare_id!("C2WJzwp5XysqRm5PQuM6ZTAeYxxhRyUWf3UJnZjVqMV5");
pub const PROTOCOL_VERSION: u8 = 2;

pub const GROTH16_PROOF_SIZE: usize = 256;
pub const SOL_ASSET_SEED: &[u8] = b"sol";

pub const PAIRING_INPUT_SIZE: usize = 768;
pub mod groth16 {
    use super::*;
    use solana_bn254::prelude::{
        alt_bn128_addition,
        alt_bn128_multiplication,
        alt_bn128_pairing,
        ALT_BN128_ADDITION_OUTPUT_LEN,
    };

    pub type G1 = [u8; 64];
    pub type G2 = [u8; 128];

    pub struct Proof {
        pub a: G1,
        pub b: G2,
        pub c: G1,
    }

    pub struct VK {
        pub alpha_g1: G1,
        pub beta_g2: G2,
        pub gamma_g2: G2,
        pub delta_g2: G2,
        pub ic: &'static [G1],
    }

    pub fn parse_proof(data: &[u8; GROTH16_PROOF_SIZE]) -> Proof {
        let mut a = [0u8; 64];
        let mut b = [0u8; 128];
        let mut c = [0u8; 64];
        a.copy_from_slice(&data[0..64]);
        b.copy_from_slice(&data[64..192]);
        c.copy_from_slice(&data[192..256]);
        Proof { a, b, c }
    }

    fn negate_g1(point: &G1) -> Result<G1> {
        let field_modulus: [u8; 32] = [
            0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29,
            0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58, 0x5d,
            0x97, 0x81, 0x6a, 0x91, 0x68, 0x71, 0xca, 0x8d,
            0x3c, 0x20, 0x8c, 0x16, 0xd8, 0x7c, 0xfd, 0x47,
        ];

        let mut result = *point;

        let mut borrow: u16 = 0;
        for i in (0..32).rev() {
            let diff = (field_modulus[i] as u16)
                .wrapping_sub(point[32 + i] as u16)
                .wrapping_sub(borrow);
            result[32 + i] = diff as u8;
            borrow = if diff > 255 { 1 } else { 0 };
        }

        Ok(result)
    }

    pub fn verify(
        proof: &Proof,
        public_inputs: &[&[u8; 32]],
        vk: &VK,
    ) -> Result<bool> {
        require!(
            public_inputs.len() + 1 == vk.ic.len(),
            SentinelError::InvalidProofInputs
        );

        let mut vk_x: [u8; 64] = vk.ic[0];

        for (i, input) in public_inputs.iter().enumerate() {
            let mut mul_input = [0u8; 96];
            mul_input[..64].copy_from_slice(&vk.ic[i + 1]);
            mul_input[64..96].copy_from_slice(*input);

            let mul_result = alt_bn128_multiplication(&mul_input)
                .map_err(|_| error!(SentinelError::ProofVerificationFailed))?;

            let mut add_input = [0u8; 128];
            add_input[..64].copy_from_slice(&vk_x);
            add_input[64..128].copy_from_slice(
                &mul_result[..ALT_BN128_ADDITION_OUTPUT_LEN],
            );

            let add_result = alt_bn128_addition(&add_input)
                .map_err(|_| error!(SentinelError::ProofVerificationFailed))?;

            vk_x.copy_from_slice(&add_result[..64]);
        }

        let neg_a = negate_g1(&proof.a)?;

        let mut pairing_input = [0u8; PAIRING_INPUT_SIZE];

        pairing_input[0..64].copy_from_slice(&neg_a);
        pairing_input[64..192].copy_from_slice(&proof.b);

        pairing_input[192..256].copy_from_slice(&vk.alpha_g1);
        pairing_input[256..384].copy_from_slice(&vk.beta_g2);

        pairing_input[384..448].copy_from_slice(&vk_x);
        pairing_input[448..576].copy_from_slice(&vk.gamma_g2);

        pairing_input[576..640].copy_from_slice(&proof.c);
        pairing_input[640..768].copy_from_slice(&vk.delta_g2);

        let pairing_result = alt_bn128_pairing(&pairing_input)
            .map_err(|_| error!(SentinelError::ProofVerificationFailed))?;

        let valid = pairing_result[31] == 1
            && pairing_result[..31].iter().all(|&b| b == 0);

        Ok(valid)
    }
}
pub mod vk {
    use super::groth16::{G1, VK};

    static COMMITMENT_IC: [G1; 3] = [
        [
            0x22, 0xbb, 0x4a, 0xae, 0xfb, 0x33, 0x24, 0x55, 0xca, 0xe8, 0x7b, 0xc5, 0x5b, 0xe8, 0x22, 0x51,
            0x36, 0x86, 0x7d, 0x99, 0x66, 0xdc, 0x25, 0x8e, 0xd0, 0xf8, 0x50, 0x8c, 0x98, 0x3c, 0xa3, 0x1d,
            0x12, 0x91, 0xdd, 0x55, 0x17, 0x0d, 0xfd, 0xa7, 0x6f, 0x73, 0x82, 0x2c, 0xed, 0x03, 0x13, 0x9f,
            0x29, 0xf0, 0xd2, 0xc3, 0xe3, 0x7c, 0x97, 0x17, 0x1c, 0xae, 0x91, 0xff, 0x86, 0x83, 0xfd, 0xf0,
        ],
        [
            0x30, 0x59, 0x30, 0x7c, 0x59, 0x30, 0xaa, 0xeb, 0x2f, 0xb8, 0x4b, 0x89, 0xdf, 0x36, 0x3f, 0x13,
            0x00, 0x9d, 0x63, 0x90, 0x96, 0xa7, 0xcc, 0xb6, 0x3c, 0x54, 0x16, 0x38, 0x0a, 0x47, 0x40, 0x31,
            0x1f, 0xb0, 0x42, 0x45, 0xbf, 0xa4, 0xe0, 0x30, 0xc3, 0xe9, 0xca, 0xaa, 0xbe, 0x79, 0xf2, 0xca,
            0x72, 0x23, 0x8f, 0x3c, 0xc3, 0x41, 0x7d, 0x39, 0x34, 0x7f, 0x3e, 0x32, 0x7e, 0xd4, 0x53, 0x46,
        ],
        [
            0x17, 0x83, 0x7a, 0xe7, 0x84, 0xa9, 0xcd, 0x7a, 0x88, 0x86, 0x22, 0xfd, 0x1c, 0xb1, 0x6d, 0x18,
            0x85, 0x2d, 0x27, 0xd3, 0xe2, 0xc8, 0x53, 0xe5, 0x84, 0x38, 0x39, 0x50, 0xee, 0xe4, 0x05, 0x7d,
            0x1c, 0x4c, 0x43, 0x7e, 0x1e, 0xad, 0x2a, 0x03, 0x69, 0x6d, 0xc6, 0xda, 0xc2, 0xee, 0x28, 0x29,
            0x06, 0xe2, 0xc6, 0xbe, 0xd2, 0xd6, 0x9e, 0x7a, 0x1a, 0x66, 0x62, 0x21, 0xf4, 0x4d, 0x6f, 0xb3,
        ],
    ];

    pub static COMMITMENT_VK: VK = VK {
        alpha_g1: [
            0x18, 0x01, 0x34, 0x26, 0x24, 0x8f, 0xb8, 0x92, 0x8a, 0x7b, 0x01, 0x05, 0x6d, 0x67, 0xed, 0xa4,
            0x16, 0x41, 0xf2, 0x8c, 0xb0, 0x87, 0xb9, 0x63, 0x97, 0x7b, 0x5b, 0x3e, 0x89, 0x9c, 0xce, 0x93,
            0x28, 0x57, 0x13, 0xa1, 0xd0, 0x30, 0x18, 0xae, 0x7a, 0xb4, 0x2f, 0xce, 0xcd, 0x34, 0x8b, 0xf3,
            0x59, 0x59, 0x36, 0x99, 0x64, 0x3b, 0x27, 0x8e, 0x17, 0xda, 0x88, 0x0b, 0xa7, 0x34, 0x0d, 0x34,
        ],
        beta_g2: [
            0x21, 0xe4, 0xd3, 0xd1, 0x21, 0xb8, 0xb3, 0xd4, 0xee, 0x59, 0x90, 0xe1, 0xb7, 0xf9, 0xb5, 0xed,
            0xf1, 0x29, 0xf7, 0x7a, 0x02, 0x9a, 0xd4, 0x2c, 0xfe, 0xde, 0x02, 0x4a, 0x8f, 0xf2, 0x34, 0x07,
            0x2e, 0x69, 0xd5, 0x03, 0x5f, 0xfb, 0x53, 0x7d, 0x39, 0x9a, 0xe2, 0x52, 0x19, 0x1d, 0xe4, 0xb0,
            0x8d, 0xee, 0xc2, 0x34, 0x11, 0x0b, 0x0b, 0xc8, 0x77, 0x7a, 0xd9, 0xd6, 0xda, 0x06, 0x99, 0xfe,
            0x2f, 0x0b, 0x0b, 0x95, 0xd2, 0x5e, 0x79, 0xb6, 0x02, 0xc3, 0x6e, 0xee, 0x0b, 0x35, 0x24, 0x87,
            0x72, 0xaa, 0xc9, 0x3a, 0x41, 0x7a, 0x5e, 0xb8, 0xe0, 0xec, 0x45, 0x64, 0xad, 0x5b, 0x60, 0x53,
            0x2f, 0x77, 0x6c, 0xed, 0xcb, 0x46, 0x7e, 0xce, 0x45, 0x15, 0x17, 0x2a, 0x5d, 0x7b, 0xc9, 0x6f,
            0x0b, 0x93, 0x0c, 0x5a, 0xd8, 0x86, 0x01, 0x38, 0xf8, 0x0f, 0x74, 0xde, 0xa8, 0xce, 0x73, 0x8b,
        ],
        gamma_g2: [
            0x19, 0x8e, 0x93, 0x93, 0x92, 0x0d, 0x48, 0x3a, 0x72, 0x60, 0xbf, 0xb7, 0x31, 0xfb, 0x5d, 0x25,
            0xf1, 0xaa, 0x49, 0x33, 0x35, 0xa9, 0xe7, 0x12, 0x97, 0xe4, 0x85, 0xb7, 0xae, 0xf3, 0x12, 0xc2,
            0x18, 0x00, 0xde, 0xef, 0x12, 0x1f, 0x1e, 0x76, 0x42, 0x6a, 0x00, 0x66, 0x5e, 0x5c, 0x44, 0x79,
            0x67, 0x43, 0x22, 0xd4, 0xf7, 0x5e, 0xda, 0xdd, 0x46, 0xde, 0xbd, 0x5c, 0xd9, 0x92, 0xf6, 0xed,
            0x09, 0x06, 0x89, 0xd0, 0x58, 0x5f, 0xf0, 0x75, 0xec, 0x9e, 0x99, 0xad, 0x69, 0x0c, 0x33, 0x95,
            0xbc, 0x4b, 0x31, 0x33, 0x70, 0xb3, 0x8e, 0xf3, 0x55, 0xac, 0xda, 0xdc, 0xd1, 0x22, 0x97, 0x5b,
            0x12, 0xc8, 0x5e, 0xa5, 0xdb, 0x8c, 0x6d, 0xeb, 0x4a, 0xab, 0x71, 0x80, 0x8d, 0xcb, 0x40, 0x8f,
            0xe3, 0xd1, 0xe7, 0x69, 0x0c, 0x43, 0xd3, 0x7b, 0x4c, 0xe6, 0xcc, 0x01, 0x66, 0xfa, 0x7d, 0xaa,
        ],
        delta_g2: [
            0x2d, 0x27, 0xc4, 0x6c, 0xbd, 0xb5, 0xa1, 0xc4, 0x0a, 0x94, 0xec, 0x80, 0x70, 0xd8, 0x71, 0x2c,
            0x27, 0x80, 0xcf, 0x3d, 0xce, 0xf4, 0xbf, 0x12, 0x90, 0x72, 0x9e, 0x70, 0x6f, 0xb9, 0xbf, 0x7b,
            0x2e, 0xc7, 0x86, 0xd3, 0xc6, 0x39, 0xf9, 0xd5, 0x7b, 0x5b, 0xcc, 0x3b, 0xc0, 0xf1, 0x4d, 0x7f,
            0x51, 0xa2, 0x62, 0x65, 0x43, 0xd1, 0x3f, 0x48, 0x26, 0x30, 0x1e, 0xab, 0xba, 0xae, 0xee, 0x73,
            0x10, 0xcf, 0x8d, 0x3a, 0xea, 0xd9, 0xfb, 0xfb, 0x56, 0xaa, 0x2f, 0xe8, 0x64, 0x3a, 0xf1, 0x72,
            0x41, 0xbb, 0xfc, 0x4b, 0x78, 0x07, 0x44, 0x1c, 0xbb, 0x5d, 0x70, 0xa9, 0x68, 0x06, 0xd5, 0x72,
            0x0d, 0xde, 0x27, 0x37, 0x6e, 0xf8, 0x0e, 0x7f, 0x38, 0xf7, 0xe2, 0x06, 0x1e, 0x7b, 0xd0, 0x51,
            0x0b, 0x68, 0x28, 0xdd, 0x91, 0x48, 0x71, 0xc4, 0xb2, 0x16, 0xd1, 0x39, 0x2d, 0xcd, 0x0e, 0x54,
        ],
        ic: &COMMITMENT_IC,
    };

    static TRANSFER_IC: [G1; 6] = [
        [
            0x03, 0x3b, 0x45, 0x11, 0x4c, 0x23, 0x5e, 0x5d, 0xe6, 0xb2, 0xb1, 0x27, 0xf3, 0xd7, 0xcc, 0x1f,
            0x11, 0x31, 0x45, 0xe8, 0x06, 0xb5, 0x6c, 0xc6, 0x6c, 0xc2, 0x72, 0xb8, 0x40, 0xb0, 0x84, 0xed,
            0x2a, 0xb1, 0x8f, 0x97, 0x93, 0xf2, 0x73, 0x03, 0x05, 0x66, 0x8f, 0x8d, 0x99, 0x41, 0x7b, 0x99,
            0xf2, 0x3d, 0x30, 0x85, 0x40, 0xc5, 0x5f, 0x78, 0x75, 0xd9, 0x70, 0x78, 0xfd, 0xe5, 0xe6, 0x65,
        ],
        [
            0x0a, 0x40, 0x72, 0x2e, 0xca, 0x7f, 0x62, 0x3e, 0x97, 0x4d, 0x96, 0x1f, 0xe3, 0xf6, 0xf5, 0xd9,
            0x24, 0x0e, 0x38, 0xc1, 0x8d, 0x89, 0xcf, 0xc7, 0xf7, 0xf4, 0x46, 0xc0, 0x93, 0xd8, 0x1c, 0x6e,
            0x18, 0x6b, 0x59, 0x31, 0x69, 0xe7, 0x56, 0x3f, 0xca, 0x5a, 0x08, 0xdf, 0xb9, 0x39, 0x70, 0xce,
            0x9c, 0xd9, 0xfb, 0x8d, 0xcb, 0xbf, 0x9b, 0x96, 0x34, 0x5c, 0x9f, 0xd4, 0xe9, 0x59, 0x53, 0x84,
        ],
        [
            0x2a, 0x6a, 0x1e, 0x6c, 0x71, 0x3e, 0x9e, 0x9d, 0x10, 0x62, 0x8c, 0xc0, 0x1a, 0x8e, 0x7a, 0x83,
            0xdb, 0x91, 0x01, 0x58, 0xdc, 0xaf, 0x73, 0xe7, 0x26, 0x64, 0xfe, 0x70, 0x1c, 0xc4, 0x5b, 0x5f,
            0x22, 0xb0, 0xbe, 0x82, 0x78, 0x10, 0x51, 0xd4, 0x67, 0x77, 0x79, 0x26, 0x78, 0x79, 0x8c, 0xec,
            0x83, 0xb6, 0x01, 0x94, 0xa7, 0x69, 0xa1, 0xbb, 0xe8, 0x99, 0x6d, 0x24, 0x8c, 0xdb, 0x0a, 0x93,
        ],
        [
            0x0d, 0x7b, 0xee, 0xf5, 0xb4, 0x30, 0x79, 0xf6, 0xaa, 0x44, 0x7b, 0xa5, 0x7c, 0x52, 0x6a, 0x2a,
            0x1e, 0x58, 0x56, 0xff, 0xe6, 0x60, 0x6f, 0x02, 0x3f, 0xe8, 0x87, 0x18, 0xa8, 0x3b, 0x37, 0x06,
            0x2c, 0x63, 0x3d, 0xb1, 0xd9, 0xf4, 0xf7, 0x44, 0x66, 0x2d, 0xca, 0xe0, 0xd1, 0x69, 0x81, 0xbb,
            0x20, 0x68, 0x62, 0xcf, 0x3d, 0xf7, 0xaa, 0x63, 0x72, 0xc0, 0x05, 0x37, 0x4a, 0x67, 0x88, 0x5c,
        ],
        [
            0x20, 0x6a, 0x05, 0x5c, 0x39, 0x1f, 0xd8, 0x9f, 0xfc, 0x07, 0x6d, 0xc3, 0x31, 0xe7, 0x28, 0x9c,
            0x32, 0x9c, 0xf6, 0x3f, 0xab, 0x74, 0x9d, 0x54, 0xc2, 0x0d, 0x1e, 0xe9, 0xa8, 0x32, 0xb7, 0xb0,
            0x03, 0xbe, 0x52, 0x82, 0x4f, 0x15, 0xc9, 0x26, 0xd2, 0x83, 0x49, 0x60, 0xd6, 0x93, 0x6b, 0x79,
            0x82, 0x79, 0x37, 0xa7, 0x58, 0x89, 0xa5, 0xc4, 0x09, 0xbf, 0x77, 0x38, 0xfa, 0xb2, 0x4e, 0x3d,
        ],
        [
            0x0c, 0x88, 0x8a, 0x5f, 0xed, 0x5e, 0x2d, 0x2e, 0x3a, 0xdf, 0x5c, 0x1e, 0x60, 0x32, 0x6e, 0xba,
            0x14, 0x39, 0x0d, 0xdc, 0xcb, 0x17, 0x49, 0xf6, 0x5b, 0x55, 0xea, 0xbf, 0x9e, 0x8a, 0x6b, 0x47,
            0x1b, 0x72, 0xe5, 0xc8, 0x03, 0xed, 0xf6, 0xf6, 0x18, 0x7a, 0xe5, 0xd4, 0xc0, 0x56, 0x24, 0xa4,
            0x82, 0xdc, 0x61, 0xd2, 0x45, 0xec, 0x26, 0x3b, 0xc9, 0x12, 0xcc, 0x7e, 0x20, 0x0f, 0xdc, 0x98,
        ],
    ];

    pub static TRANSFER_VK: VK = VK {
        alpha_g1: [
            0x18, 0x01, 0x34, 0x26, 0x24, 0x8f, 0xb8, 0x92, 0x8a, 0x7b, 0x01, 0x05, 0x6d, 0x67, 0xed, 0xa4,
            0x16, 0x41, 0xf2, 0x8c, 0xb0, 0x87, 0xb9, 0x63, 0x97, 0x7b, 0x5b, 0x3e, 0x89, 0x9c, 0xce, 0x93,
            0x28, 0x57, 0x13, 0xa1, 0xd0, 0x30, 0x18, 0xae, 0x7a, 0xb4, 0x2f, 0xce, 0xcd, 0x34, 0x8b, 0xf3,
            0x59, 0x59, 0x36, 0x99, 0x64, 0x3b, 0x27, 0x8e, 0x17, 0xda, 0x88, 0x0b, 0xa7, 0x34, 0x0d, 0x34,
        ],
        beta_g2: [
            0x21, 0xe4, 0xd3, 0xd1, 0x21, 0xb8, 0xb3, 0xd4, 0xee, 0x59, 0x90, 0xe1, 0xb7, 0xf9, 0xb5, 0xed,
            0xf1, 0x29, 0xf7, 0x7a, 0x02, 0x9a, 0xd4, 0x2c, 0xfe, 0xde, 0x02, 0x4a, 0x8f, 0xf2, 0x34, 0x07,
            0x2e, 0x69, 0xd5, 0x03, 0x5f, 0xfb, 0x53, 0x7d, 0x39, 0x9a, 0xe2, 0x52, 0x19, 0x1d, 0xe4, 0xb0,
            0x8d, 0xee, 0xc2, 0x34, 0x11, 0x0b, 0x0b, 0xc8, 0x77, 0x7a, 0xd9, 0xd6, 0xda, 0x06, 0x99, 0xfe,
            0x2f, 0x0b, 0x0b, 0x95, 0xd2, 0x5e, 0x79, 0xb6, 0x02, 0xc3, 0x6e, 0xee, 0x0b, 0x35, 0x24, 0x87,
            0x72, 0xaa, 0xc9, 0x3a, 0x41, 0x7a, 0x5e, 0xb8, 0xe0, 0xec, 0x45, 0x64, 0xad, 0x5b, 0x60, 0x53,
            0x2f, 0x77, 0x6c, 0xed, 0xcb, 0x46, 0x7e, 0xce, 0x45, 0x15, 0x17, 0x2a, 0x5d, 0x7b, 0xc9, 0x6f,
            0x0b, 0x93, 0x0c, 0x5a, 0xd8, 0x86, 0x01, 0x38, 0xf8, 0x0f, 0x74, 0xde, 0xa8, 0xce, 0x73, 0x8b,
        ],
        gamma_g2: [
            0x19, 0x8e, 0x93, 0x93, 0x92, 0x0d, 0x48, 0x3a, 0x72, 0x60, 0xbf, 0xb7, 0x31, 0xfb, 0x5d, 0x25,
            0xf1, 0xaa, 0x49, 0x33, 0x35, 0xa9, 0xe7, 0x12, 0x97, 0xe4, 0x85, 0xb7, 0xae, 0xf3, 0x12, 0xc2,
            0x18, 0x00, 0xde, 0xef, 0x12, 0x1f, 0x1e, 0x76, 0x42, 0x6a, 0x00, 0x66, 0x5e, 0x5c, 0x44, 0x79,
            0x67, 0x43, 0x22, 0xd4, 0xf7, 0x5e, 0xda, 0xdd, 0x46, 0xde, 0xbd, 0x5c, 0xd9, 0x92, 0xf6, 0xed,
            0x09, 0x06, 0x89, 0xd0, 0x58, 0x5f, 0xf0, 0x75, 0xec, 0x9e, 0x99, 0xad, 0x69, 0x0c, 0x33, 0x95,
            0xbc, 0x4b, 0x31, 0x33, 0x70, 0xb3, 0x8e, 0xf3, 0x55, 0xac, 0xda, 0xdc, 0xd1, 0x22, 0x97, 0x5b,
            0x12, 0xc8, 0x5e, 0xa5, 0xdb, 0x8c, 0x6d, 0xeb, 0x4a, 0xab, 0x71, 0x80, 0x8d, 0xcb, 0x40, 0x8f,
            0xe3, 0xd1, 0xe7, 0x69, 0x0c, 0x43, 0xd3, 0x7b, 0x4c, 0xe6, 0xcc, 0x01, 0x66, 0xfa, 0x7d, 0xaa,
        ],
        delta_g2: [
            0x22, 0x39, 0xfe, 0x17, 0x28, 0x0c, 0x88, 0x1e, 0x65, 0x91, 0xd5, 0x70, 0xa6, 0x91, 0x44, 0x79,
            0x10, 0x27, 0xc2, 0x3b, 0x0e, 0x68, 0x2a, 0x34, 0x1b, 0x9e, 0x18, 0xd8, 0xf6, 0x99, 0x65, 0xa5,
            0x06, 0x15, 0x3d, 0x50, 0xe8, 0xb4, 0x05, 0xb2, 0x14, 0xf0, 0x6a, 0x47, 0x47, 0x6f, 0xc4, 0xce,
            0xa2, 0x8e, 0x79, 0x74, 0x16, 0x59, 0x10, 0x4c, 0xe0, 0xed, 0x26, 0x27, 0x09, 0xc9, 0x40, 0x67,
            0x1a, 0x76, 0x58, 0xf0, 0x82, 0xef, 0xd1, 0x2d, 0x0e, 0xdb, 0x8a, 0x8d, 0x42, 0x68, 0xf9, 0x95,
            0x67, 0x8e, 0x3b, 0x8c, 0xbb, 0x8f, 0xd9, 0x82, 0xc2, 0x4e, 0xd1, 0x37, 0x2b, 0xcf, 0xf8, 0xe9,
            0x1c, 0x10, 0xac, 0x56, 0x2e, 0x7c, 0x83, 0xc6, 0xe6, 0x9b, 0xfa, 0xbc, 0x28, 0xf8, 0x78, 0xe1,
            0x4a, 0xf1, 0x1a, 0x61, 0x14, 0x61, 0x42, 0xb5, 0xe1, 0xa5, 0xae, 0x39, 0x63, 0x1b, 0xcc, 0x8b,
        ],
        ic: &TRANSFER_IC,
    };

    static WITHDRAW_IC: [G1; 5] = [
        [
            0x28, 0x20, 0xad, 0x6e, 0x00, 0xc8, 0x51, 0xc7, 0x2a, 0xc8, 0xde, 0x10, 0xc1, 0xba, 0xf0, 0x84,
            0x44, 0xf3, 0xd2, 0x37, 0x79, 0xdf, 0xe2, 0xc2, 0x24, 0x37, 0xfc, 0x9c, 0x38, 0xc4, 0x9e, 0x7f,
            0x07, 0x86, 0xff, 0xf2, 0x22, 0x6e, 0x35, 0x9d, 0xc0, 0xd4, 0x4c, 0xbd, 0x7f, 0xf1, 0xc3, 0xb3,
            0xbd, 0x2e, 0x4f, 0x41, 0x75, 0x80, 0xa8, 0x3d, 0x5c, 0x41, 0x69, 0x81, 0xfd, 0xb3, 0x25, 0x59,
        ],
        [
            0x15, 0x9e, 0x42, 0xab, 0x34, 0xb5, 0x46, 0xbb, 0xb4, 0xa5, 0x81, 0xb1, 0xaa, 0x69, 0xd6, 0x3f,
            0x29, 0x58, 0x17, 0x4b, 0x19, 0x87, 0x56, 0x68, 0x7d, 0x3f, 0x69, 0x63, 0x51, 0x82, 0xf0, 0x88,
            0x19, 0xd3, 0x22, 0x63, 0x51, 0x0c, 0xcd, 0xb2, 0xe7, 0xde, 0x90, 0x5e, 0xd0, 0x96, 0x4b, 0xa8,
            0xaf, 0xb5, 0xa6, 0x2a, 0x79, 0x20, 0x42, 0xa1, 0xf8, 0xca, 0xbf, 0x7f, 0x69, 0xed, 0xf2, 0x13,
        ],
        [
            0x00, 0x08, 0x81, 0x34, 0x0b, 0xae, 0x5e, 0x4c, 0x68, 0x6a, 0xfc, 0x82, 0x58, 0x1e, 0x04, 0xdb,
            0xfb, 0xf6, 0x62, 0x38, 0xcc, 0x97, 0x82, 0x72, 0xb2, 0xea, 0x11, 0x3c, 0x4c, 0x1e, 0x0f, 0x44,
            0x27, 0x61, 0x26, 0x9e, 0xde, 0xac, 0x21, 0x0e, 0x2b, 0x31, 0x24, 0x9e, 0xf2, 0xc7, 0x8d, 0xcd,
            0xc6, 0x1e, 0xd0, 0xe0, 0x1f, 0x87, 0xe2, 0x75, 0xf4, 0xc4, 0xf5, 0x21, 0x84, 0xe6, 0x3c, 0x6b,
        ],
        [
            0x12, 0x85, 0x70, 0xa8, 0xc4, 0x0f, 0xab, 0x84, 0x79, 0x76, 0xed, 0x6f, 0xde, 0xc9, 0xc2, 0x51,
            0xad, 0x0f, 0x91, 0x2c, 0xfa, 0x6e, 0x2f, 0x84, 0x57, 0x07, 0x01, 0xc7, 0x04, 0xf2, 0x11, 0x07,
            0x22, 0x20, 0xf8, 0xf4, 0xb0, 0xad, 0x83, 0x7a, 0x97, 0x67, 0xdb, 0xb3, 0xee, 0xaa, 0xaf, 0x8f,
            0x13, 0xc0, 0xfa, 0xd3, 0xb6, 0x40, 0xe8, 0xdd, 0x60, 0x6d, 0x6b, 0xcd, 0xb0, 0x8a, 0x41, 0x90,
        ],
        [
            0x1f, 0xf4, 0x6b, 0x4e, 0xde, 0xe3, 0x51, 0xdc, 0xda, 0x7d, 0xbe, 0xbc, 0x79, 0x02, 0x1f, 0x47,
            0x31, 0xb8, 0x1e, 0x94, 0x72, 0xa2, 0xa2, 0x57, 0xdb, 0x39, 0x63, 0xb8, 0xa9, 0x3f, 0xcf, 0xf3,
            0x2c, 0x38, 0x47, 0xaf, 0x21, 0xb7, 0xb7, 0xae, 0x84, 0x22, 0x05, 0x74, 0x0e, 0x84, 0xd8, 0x44,
            0xb6, 0x3c, 0xbf, 0x76, 0x50, 0xa8, 0x75, 0xf8, 0x72, 0x19, 0x1a, 0x35, 0x5f, 0xb0, 0xd8, 0x9a,
        ],
    ];

    pub static WITHDRAW_VK: VK = VK {
        alpha_g1: [
            0x18, 0x01, 0x34, 0x26, 0x24, 0x8f, 0xb8, 0x92, 0x8a, 0x7b, 0x01, 0x05, 0x6d, 0x67, 0xed, 0xa4,
            0x16, 0x41, 0xf2, 0x8c, 0xb0, 0x87, 0xb9, 0x63, 0x97, 0x7b, 0x5b, 0x3e, 0x89, 0x9c, 0xce, 0x93,
            0x28, 0x57, 0x13, 0xa1, 0xd0, 0x30, 0x18, 0xae, 0x7a, 0xb4, 0x2f, 0xce, 0xcd, 0x34, 0x8b, 0xf3,
            0x59, 0x59, 0x36, 0x99, 0x64, 0x3b, 0x27, 0x8e, 0x17, 0xda, 0x88, 0x0b, 0xa7, 0x34, 0x0d, 0x34,
        ],
        beta_g2: [
            0x21, 0xe4, 0xd3, 0xd1, 0x21, 0xb8, 0xb3, 0xd4, 0xee, 0x59, 0x90, 0xe1, 0xb7, 0xf9, 0xb5, 0xed,
            0xf1, 0x29, 0xf7, 0x7a, 0x02, 0x9a, 0xd4, 0x2c, 0xfe, 0xde, 0x02, 0x4a, 0x8f, 0xf2, 0x34, 0x07,
            0x2e, 0x69, 0xd5, 0x03, 0x5f, 0xfb, 0x53, 0x7d, 0x39, 0x9a, 0xe2, 0x52, 0x19, 0x1d, 0xe4, 0xb0,
            0x8d, 0xee, 0xc2, 0x34, 0x11, 0x0b, 0x0b, 0xc8, 0x77, 0x7a, 0xd9, 0xd6, 0xda, 0x06, 0x99, 0xfe,
            0x2f, 0x0b, 0x0b, 0x95, 0xd2, 0x5e, 0x79, 0xb6, 0x02, 0xc3, 0x6e, 0xee, 0x0b, 0x35, 0x24, 0x87,
            0x72, 0xaa, 0xc9, 0x3a, 0x41, 0x7a, 0x5e, 0xb8, 0xe0, 0xec, 0x45, 0x64, 0xad, 0x5b, 0x60, 0x53,
            0x2f, 0x77, 0x6c, 0xed, 0xcb, 0x46, 0x7e, 0xce, 0x45, 0x15, 0x17, 0x2a, 0x5d, 0x7b, 0xc9, 0x6f,
            0x0b, 0x93, 0x0c, 0x5a, 0xd8, 0x86, 0x01, 0x38, 0xf8, 0x0f, 0x74, 0xde, 0xa8, 0xce, 0x73, 0x8b,
        ],
        gamma_g2: [
            0x19, 0x8e, 0x93, 0x93, 0x92, 0x0d, 0x48, 0x3a, 0x72, 0x60, 0xbf, 0xb7, 0x31, 0xfb, 0x5d, 0x25,
            0xf1, 0xaa, 0x49, 0x33, 0x35, 0xa9, 0xe7, 0x12, 0x97, 0xe4, 0x85, 0xb7, 0xae, 0xf3, 0x12, 0xc2,
            0x18, 0x00, 0xde, 0xef, 0x12, 0x1f, 0x1e, 0x76, 0x42, 0x6a, 0x00, 0x66, 0x5e, 0x5c, 0x44, 0x79,
            0x67, 0x43, 0x22, 0xd4, 0xf7, 0x5e, 0xda, 0xdd, 0x46, 0xde, 0xbd, 0x5c, 0xd9, 0x92, 0xf6, 0xed,
            0x09, 0x06, 0x89, 0xd0, 0x58, 0x5f, 0xf0, 0x75, 0xec, 0x9e, 0x99, 0xad, 0x69, 0x0c, 0x33, 0x95,
            0xbc, 0x4b, 0x31, 0x33, 0x70, 0xb3, 0x8e, 0xf3, 0x55, 0xac, 0xda, 0xdc, 0xd1, 0x22, 0x97, 0x5b,
            0x12, 0xc8, 0x5e, 0xa5, 0xdb, 0x8c, 0x6d, 0xeb, 0x4a, 0xab, 0x71, 0x80, 0x8d, 0xcb, 0x40, 0x8f,
            0xe3, 0xd1, 0xe7, 0x69, 0x0c, 0x43, 0xd3, 0x7b, 0x4c, 0xe6, 0xcc, 0x01, 0x66, 0xfa, 0x7d, 0xaa,
        ],
        delta_g2: [
            0x18, 0xab, 0xb4, 0x17, 0xde, 0x0a, 0x41, 0x2a, 0x2e, 0x3b, 0x9b, 0x71, 0xfe, 0x44, 0x45, 0x75,
            0x91, 0x41, 0x04, 0x0e, 0x70, 0xc8, 0x9c, 0xed, 0x34, 0x73, 0x87, 0x19, 0x04, 0xf1, 0xf5, 0xb7,
            0x24, 0xd6, 0x14, 0x1d, 0xe9, 0x9a, 0x01, 0x08, 0x3c, 0x28, 0x3c, 0x40, 0xcd, 0x2d, 0xae, 0x37,
            0x0e, 0xd3, 0x82, 0x38, 0x2b, 0x28, 0x4d, 0xc0, 0x4a, 0xeb, 0xec, 0x4d, 0x8c, 0xa7, 0x30, 0x6e,
            0x02, 0xdf, 0xad, 0xdc, 0x9e, 0x4b, 0x6e, 0xff, 0x46, 0x40, 0x0e, 0xb2, 0x82, 0xec, 0x17, 0xb0,
            0x90, 0x8c, 0xd4, 0xf6, 0xcb, 0xa0, 0xbd, 0x55, 0x27, 0xe8, 0x40, 0x12, 0x9d, 0xb6, 0x56, 0x86,
            0x13, 0x6b, 0x5f, 0xdb, 0x06, 0xe2, 0x57, 0x58, 0x6c, 0x89, 0x51, 0x85, 0x45, 0xe1, 0xb9, 0xa4,
            0x5f, 0xfd, 0x8e, 0xab, 0xe8, 0xe6, 0x41, 0xb2, 0x2c, 0xc7, 0x65, 0x28, 0xeb, 0x5d, 0x9f, 0xb7,
        ],
        ic: &WITHDRAW_IC,
    };
}

fn amount_to_field(amount: u64) -> [u8; 32] {
    let mut field = [0u8; 32];
    field[..8].copy_from_slice(&amount.to_le_bytes());
    field
}

fn sol_asset_key() -> [u8; 32] {
    [0u8; 32]
}

fn mint_asset_key(mint: &Pubkey) -> [u8; 32] {
    mint.to_bytes()
}

fn ensure_vault_pool_initialized<'info>(
    payer: &Signer<'info>,
    vault_pool: &AccountInfo<'info>,
    system_program: &Program<'info, System>,
    rail_key: &Pubkey,
    bump: u8,
) -> Result<()> {
    if vault_pool.lamports() > 0 {
        return Ok(());
    }

    let rent = Rent::get()?;
    let seeds: &[&[u8]] = &[b"vault_pool", rail_key.as_ref(), &[bump]];
    let signer_seeds = &[seeds];

    let ix = anchor_lang::solana_program::system_instruction::create_account(
        &payer.key(),
        &vault_pool.key(),
        rent.minimum_balance(0),
        0,
        &crate::ID,
    );

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            payer.to_account_info(),
            vault_pool.clone(),
            system_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    Ok(())
}

#[program]
pub mod sentinel {
    use super::*;
    
    pub fn initialize_rail(
        ctx: Context<InitializeRail>,
        institution_type: u8,
        compliance_level: u8,
    ) -> Result<()> {
        require!(
            ctx.accounts.authority_token_account.amount > 0,
            SentinelError::InsufficientNorthTokens
        );

        let rail = &mut ctx.accounts.rail;
        let clock = Clock::get()?;

        rail.authority = ctx.accounts.authority.key();
        rail.institution_type = institution_type;
        rail.compliance_level = compliance_level;
        rail.is_sealed = false;
        rail.is_active = true;
        rail.is_paused = false;
        rail.total_handshakes = 0;
        rail.created_at = clock.unix_timestamp;
        rail.sealed_at = 0;
        rail.deactivated_at = 0;
        rail.version = PROTOCOL_VERSION;
        rail.audit_seal = [0u8; 32];
        rail.deactivation_reason = 0;

        Ok(())
    }
    
    pub fn initialize_zk_vault(
        ctx: Context<InitializeZkVault>,
        elgamal_pubkey: [u8; 32],
    ) -> Result<()> {
        let zk_vault = &mut ctx.accounts.zk_vault;

        zk_vault.rail = ctx.accounts.rail.key();
        zk_vault.elgamal_pubkey = elgamal_pubkey;
        zk_vault.encrypted_balance = [0u8; 64];
        zk_vault.balance_commitment = [0u8; 32];
        zk_vault.deposit_count = 0;
        zk_vault.token_deposit_count = 0;
        zk_vault.bump = ctx.bumps.zk_vault;

        Ok(())
    }
    
    pub fn create_handshake(
        ctx: Context<CreateHandshake>,
        commitment: [u8; 32],
        nullifier_hash: [u8; 32],
    ) -> Result<()> {
        require!(!ctx.accounts.rail.is_sealed, SentinelError::RailSealed);
        require!(ctx.accounts.rail.is_active, SentinelError::RailInactive);
        require!(!ctx.accounts.rail.is_paused, SentinelError::RailPaused);
        require!(
            !ctx.accounts.nullifier_registry.is_spent,
            SentinelError::NullifierAlreadyUsed
        );

        let clock = Clock::get()?;
        let rail_key = ctx.accounts.rail.key();

        let handshake = &mut ctx.accounts.handshake;
        handshake.rail = rail_key;
        handshake.commitment = commitment;
        handshake.nullifier_hash = nullifier_hash;
        handshake.is_active = true;
        handshake.created_at = clock.unix_timestamp;
        handshake.revoked_at = 0;

        let nullifier_registry = &mut ctx.accounts.nullifier_registry;
        nullifier_registry.rail = rail_key;
        nullifier_registry.nullifier_hash = nullifier_hash;
        nullifier_registry.is_spent = true;
        nullifier_registry.spent_at = clock.unix_timestamp;

        let rail = &mut ctx.accounts.rail;
        rail.total_handshakes = rail
            .total_handshakes
            .checked_add(1)
            .ok_or(SentinelError::Overflow)?;

        Ok(())
    }

    pub fn seal_rail(ctx: Context<SealRail>, audit_seal: [u8; 32]) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        require!(rail.is_active, SentinelError::RailInactive);
        require!(!rail.is_sealed, SentinelError::RailAlreadySealed);
        let clock = Clock::get()?;
        rail.audit_seal = audit_seal;
        rail.is_sealed = true;
        rail.sealed_at = clock.unix_timestamp;
        Ok(())
    }

    pub fn deactivate_rail(ctx: Context<DeactivateRail>, reason_code: u8) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        let clock = Clock::get()?;
        require!(rail.is_active, SentinelError::RailAlreadyDeactivated);
        rail.is_active = false;
        rail.deactivated_at = clock.unix_timestamp;
        rail.deactivation_reason = reason_code;
        Ok(())
    }

    pub fn pause_rail(ctx: Context<PauseRail>) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        require!(rail.is_active, SentinelError::RailInactive);
        require!(!rail.is_paused, SentinelError::RailAlreadyPaused);
        rail.is_paused = true;
        Ok(())
    }

    pub fn unpause_rail(ctx: Context<UnpauseRail>) -> Result<()> {
        let rail = &mut ctx.accounts.rail;
        require!(rail.is_active, SentinelError::RailInactive);
        require!(rail.is_paused, SentinelError::RailNotPaused);
        rail.is_paused = false;
        Ok(())
    }

    pub fn revoke_handshake(ctx: Context<RevokeHandshake>, _reason_code: u8) -> Result<()> {
        let handshake = &mut ctx.accounts.handshake;
        let rail = &ctx.accounts.rail;
        require!(handshake.is_active, SentinelError::HandshakeAlreadyRevoked);
        require!(handshake.rail == rail.key(), SentinelError::InvalidRail);
        let clock = Clock::get()?;
        handshake.is_active = false;
        handshake.revoked_at = clock.unix_timestamp;
        Ok(())
    }
    
    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
        proof: [u8; GROTH16_PROOF_SIZE],
        commitment: [u8; 32],
        nullifier_hash: [u8; 32],
        encrypted_amount: [u8; 64],
    ) -> Result<()> {
        require!(amount > 0, SentinelError::InvalidAmount);
        require!(ctx.accounts.rail.is_active, SentinelError::RailInactive);
        require!(!ctx.accounts.rail.is_paused, SentinelError::RailPaused);

        let groth16_proof = groth16::parse_proof(&proof);
        let valid = groth16::verify(
            &groth16_proof,
            &[&commitment, &nullifier_hash],
            &vk::COMMITMENT_VK,
        )?;
        require!(valid, SentinelError::InvalidZkProof);

        ensure_vault_pool_initialized(
            &ctx.accounts.sender,
            &ctx.accounts.vault_pool.to_account_info(),
            &ctx.accounts.system_program,
            &ctx.accounts.rail.key(),
            ctx.bumps.vault_pool,
        )?;

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.sender.to_account_info(),
                    to: ctx.accounts.vault_pool.to_account_info(),
                },
            ),
            amount,
        )?;

        let deposit_record = &mut ctx.accounts.deposit_record;
        let clock = Clock::get()?;
        deposit_record.rail = ctx.accounts.rail.key();
        deposit_record.sender = ctx.accounts.sender.key();
        deposit_record.encrypted_amount = encrypted_amount;
        deposit_record.commitment = commitment;
        deposit_record.is_withdrawn = false;
        deposit_record.created_at = clock.unix_timestamp;
        deposit_record.withdrawn_at = 0;
        deposit_record.bump = ctx.bumps.deposit_record;

        let expected_asset = sol_asset_key();
        let sol_asset_state = &mut ctx.accounts.sol_asset_state;
        if sol_asset_state.rail == Pubkey::default() {
            sol_asset_state.rail = ctx.accounts.rail.key();
            sol_asset_state.asset_key = expected_asset;
            sol_asset_state.bump = ctx.bumps.sol_asset_state;
        } else {
            require!(
                sol_asset_state.rail == ctx.accounts.rail.key()
                    && sol_asset_state.asset_key == expected_asset,
                SentinelError::InvalidAssetState
            );
        }
        sol_asset_state.balance_commitment = commitment;
        sol_asset_state.encrypted_balance = encrypted_amount;
        sol_asset_state.updated_at = clock.unix_timestamp;

        let zk_vault = &mut ctx.accounts.zk_vault;
        zk_vault.deposit_count = zk_vault
            .deposit_count
            .checked_add(1)
            .ok_or(SentinelError::Overflow)?;

        Ok(())
    }
    
    pub fn confidential_transfer(
        ctx: Context<ConfidentialTransfer>,
        transfer_nonce: i64,
        proof: [u8; GROTH16_PROOF_SIZE],
        sender_commitment_before: [u8; 32],
        sender_commitment_after: [u8; 32],
        receiver_commitment_before: [u8; 32],
        receiver_commitment_after: [u8; 32],
        nullifier_hash: [u8; 32],
        new_sender_encrypted_balance: [u8; 64],
        new_receiver_encrypted_balance: [u8; 64],
    ) -> Result<()> {
        require!(
            ctx.accounts.sender_rail.is_active,
            SentinelError::RailInactive
        );
        require!(
            !ctx.accounts.sender_rail.is_paused,
            SentinelError::RailPaused
        );
        require!(
            transfer_nonce > 0,
            SentinelError::InvalidTransferNonce
        );

        require!(
            ctx.accounts.sender_sol_asset_state.balance_commitment == sender_commitment_before,
            SentinelError::CommitmentMismatch
        );
        require!(
            ctx.accounts.receiver_sol_asset_state.balance_commitment == receiver_commitment_before,
            SentinelError::CommitmentMismatch
        );

        let groth16_proof = groth16::parse_proof(&proof);
        let valid = groth16::verify(
            &groth16_proof,
            &[
                &sender_commitment_before,
                &sender_commitment_after,
                &receiver_commitment_before,
                &receiver_commitment_after,
                &nullifier_hash,
            ],
            &vk::TRANSFER_VK,
        )?;
        require!(valid, SentinelError::InvalidZkProof);

        let sender_vault = &mut ctx.accounts.sender_sol_asset_state;
        sender_vault.balance_commitment = sender_commitment_after;
        sender_vault.encrypted_balance = new_sender_encrypted_balance;

        let receiver_vault = &mut ctx.accounts.receiver_sol_asset_state;
        receiver_vault.balance_commitment = receiver_commitment_after;
        receiver_vault.encrypted_balance = new_receiver_encrypted_balance;

        let transfer_record = &mut ctx.accounts.transfer_record;
        let clock = Clock::get()?;
        transfer_record.sender_rail = ctx.accounts.sender_rail.key();
        transfer_record.receiver_rail = ctx.accounts.receiver_rail.key();
        transfer_record.sender_commitment = sender_commitment_after;
        transfer_record.receiver_commitment = receiver_commitment_after;
        transfer_record.nullifier_hash = nullifier_hash;
        transfer_record.proof_hash.copy_from_slice(&proof[..32]);
        transfer_record.is_token = false;
        transfer_record.token_mint = Pubkey::default();
        transfer_record.created_at = clock.unix_timestamp;
        transfer_record.bump = ctx.bumps.transfer_record;

        Ok(())
    }
    
    pub fn withdraw(
        ctx: Context<Withdraw>,
        amount: u64,
        proof: [u8; GROTH16_PROOF_SIZE],
        balance_commitment_before: [u8; 32],
        balance_commitment_after: [u8; 32],
        nullifier_hash: [u8; 32],
        new_encrypted_balance: [u8; 64],
    ) -> Result<()> {
        require!(amount > 0, SentinelError::InvalidAmount);
        require!(ctx.accounts.rail.is_active, SentinelError::RailInactive);
        require!(!ctx.accounts.rail.is_paused, SentinelError::RailPaused);

        require!(
            ctx.accounts.sol_asset_state.balance_commitment == balance_commitment_before,
            SentinelError::CommitmentMismatch
        );

        let amount_field = amount_to_field(amount);
        let groth16_proof = groth16::parse_proof(&proof);
        let valid = groth16::verify(
            &groth16_proof,
            &[
                &balance_commitment_before,
                &balance_commitment_after,
                &amount_field,
                &nullifier_hash,
            ],
            &vk::WITHDRAW_VK,
        )?;
        require!(valid, SentinelError::InvalidZkProof);

        let vault_pool = ctx.accounts.vault_pool.to_account_info();
        let receiver = ctx.accounts.receiver.to_account_info();

        require!(
            vault_pool.lamports() >= amount,
            SentinelError::InsufficientVaultBalance
        );
        **vault_pool.try_borrow_mut_lamports()? -= amount;
        **receiver.try_borrow_mut_lamports()? += amount;

        let clock = Clock::get()?;
        let sol_asset_state = &mut ctx.accounts.sol_asset_state;
        sol_asset_state.balance_commitment = balance_commitment_after;
        sol_asset_state.encrypted_balance = new_encrypted_balance;
        sol_asset_state.updated_at = clock.unix_timestamp;

        let deposit_record = &mut ctx.accounts.deposit_record;
        require!(!deposit_record.is_withdrawn, SentinelError::AlreadyWithdrawn);
        deposit_record.is_withdrawn = true;
        deposit_record.withdrawn_at = clock.unix_timestamp;

        Ok(())
    }
    
    pub fn deposit_token(
        ctx: Context<DepositToken>,
        amount: u64,
        proof: [u8; GROTH16_PROOF_SIZE],
        commitment: [u8; 32],
        nullifier_hash: [u8; 32],
        encrypted_amount: [u8; 64],
    ) -> Result<()> {
        require!(amount > 0, SentinelError::InvalidAmount);
        require!(ctx.accounts.rail.is_active, SentinelError::RailInactive);
        require!(!ctx.accounts.rail.is_paused, SentinelError::RailPaused);

        let groth16_proof = groth16::parse_proof(&proof);
        let valid = groth16::verify(
            &groth16_proof,
            &[&commitment, &nullifier_hash],
            &vk::COMMITMENT_VK,
        )?;
        require!(valid, SentinelError::InvalidZkProof);

        let decimals = ctx.accounts.token_mint.decimals;
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.sender_token_account.to_account_info(),
                    mint: ctx.accounts.token_mint.to_account_info(),
                    to: ctx.accounts.vault_token_account.to_account_info(),
                    authority: ctx.accounts.sender.to_account_info(),
                },
            ),
            amount,
            decimals,
        )?;

        let deposit_record = &mut ctx.accounts.token_deposit_record;
        let clock = Clock::get()?;
        deposit_record.rail = ctx.accounts.rail.key();
        deposit_record.sender = ctx.accounts.sender.key();
        deposit_record.token_mint = ctx.accounts.token_mint.key();
        deposit_record.encrypted_amount = encrypted_amount;
        deposit_record.commitment = commitment;
        deposit_record.decimals = decimals;
        deposit_record.is_withdrawn = false;
        deposit_record.created_at = clock.unix_timestamp;
        deposit_record.withdrawn_at = 0;
        deposit_record.bump = ctx.bumps.token_deposit_record;

        let expected_asset = mint_asset_key(&ctx.accounts.token_mint.key());
        let token_asset_state = &mut ctx.accounts.token_asset_state;
        if token_asset_state.rail == Pubkey::default() {
            token_asset_state.rail = ctx.accounts.rail.key();
            token_asset_state.asset_key = expected_asset;
            token_asset_state.bump = ctx.bumps.token_asset_state;
        } else {
            require!(
                token_asset_state.rail == ctx.accounts.rail.key()
                    && token_asset_state.asset_key == expected_asset,
                SentinelError::InvalidAssetState
            );
        }
        token_asset_state.balance_commitment = commitment;
        token_asset_state.encrypted_balance = encrypted_amount;
        token_asset_state.updated_at = clock.unix_timestamp;

        let zk_vault = &mut ctx.accounts.zk_vault;
        zk_vault.token_deposit_count = zk_vault
            .token_deposit_count
            .checked_add(1)
            .ok_or(SentinelError::Overflow)?;

        Ok(())
    }
    
    pub fn confidential_transfer_token(
        ctx: Context<ConfidentialTransferToken>,
        transfer_nonce: i64,
        proof: [u8; GROTH16_PROOF_SIZE],
        sender_commitment_before: [u8; 32],
        sender_commitment_after: [u8; 32],
        receiver_commitment_before: [u8; 32],
        receiver_commitment_after: [u8; 32],
        nullifier_hash: [u8; 32],
        new_sender_encrypted_balance: [u8; 64],
        new_receiver_encrypted_balance: [u8; 64],
    ) -> Result<()> {
        require!(
            ctx.accounts.sender_rail.is_active,
            SentinelError::RailInactive
        );
        require!(
            !ctx.accounts.sender_rail.is_paused,
            SentinelError::RailPaused
        );
        require!(
            transfer_nonce > 0,
            SentinelError::InvalidTransferNonce
        );

        require!(
            ctx.accounts.sender_token_asset_state.balance_commitment == sender_commitment_before,
            SentinelError::CommitmentMismatch
        );
        require!(
            ctx.accounts.receiver_token_asset_state.balance_commitment == receiver_commitment_before,
            SentinelError::CommitmentMismatch
        );

        let groth16_proof = groth16::parse_proof(&proof);
        let valid = groth16::verify(
            &groth16_proof,
            &[
                &sender_commitment_before,
                &sender_commitment_after,
                &receiver_commitment_before,
                &receiver_commitment_after,
                &nullifier_hash,
            ],
            &vk::TRANSFER_VK,
        )?;
        require!(valid, SentinelError::InvalidZkProof);
        
        let sender_vault = &mut ctx.accounts.sender_token_asset_state;
        sender_vault.balance_commitment = sender_commitment_after;
        sender_vault.encrypted_balance = new_sender_encrypted_balance;

        let receiver_vault = &mut ctx.accounts.receiver_token_asset_state;
        receiver_vault.balance_commitment = receiver_commitment_after;
        receiver_vault.encrypted_balance = new_receiver_encrypted_balance;

        let transfer_record = &mut ctx.accounts.transfer_record;
        let clock = Clock::get()?;
        transfer_record.sender_rail = ctx.accounts.sender_rail.key();
        transfer_record.receiver_rail = ctx.accounts.receiver_rail.key();
        transfer_record.sender_commitment = sender_commitment_after;
        transfer_record.receiver_commitment = receiver_commitment_after;
        transfer_record.nullifier_hash = nullifier_hash;
        transfer_record.proof_hash.copy_from_slice(&proof[..32]);
        transfer_record.is_token = true;
        transfer_record.token_mint = ctx.accounts.token_mint.key();
        transfer_record.created_at = clock.unix_timestamp;
        transfer_record.bump = ctx.bumps.transfer_record;

        Ok(())
    }
    
    pub fn withdraw_token(
        ctx: Context<WithdrawToken>,
        amount: u64,
        proof: [u8; GROTH16_PROOF_SIZE],
        balance_commitment_before: [u8; 32],
        balance_commitment_after: [u8; 32],
        nullifier_hash: [u8; 32],
        new_encrypted_balance: [u8; 64],
    ) -> Result<()> {
        require!(amount > 0, SentinelError::InvalidAmount);
        require!(ctx.accounts.rail.is_active, SentinelError::RailInactive);
        require!(!ctx.accounts.rail.is_paused, SentinelError::RailPaused);

        require!(
            ctx.accounts.token_asset_state.balance_commitment == balance_commitment_before,
            SentinelError::CommitmentMismatch
        );

        let amount_field = amount_to_field(amount);
        let groth16_proof = groth16::parse_proof(&proof);
        let valid = groth16::verify(
            &groth16_proof,
            &[
                &balance_commitment_before,
                &balance_commitment_after,
                &amount_field,
                &nullifier_hash,
            ],
            &vk::WITHDRAW_VK,
        )?;
        require!(valid, SentinelError::InvalidZkProof);

        let decimals = ctx.accounts.token_mint.decimals;
        let rail_key = ctx.accounts.rail.key();
        let bump = ctx.accounts.zk_vault.bump;
        let seeds: &[&[u8]] = &[b"zk_vault", rail_key.as_ref(), &[bump]];
        let signer_seeds = &[seeds];

        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    mint: ctx.accounts.token_mint.to_account_info(),
                    to: ctx.accounts.receiver_token_account.to_account_info(),
                    authority: ctx.accounts.zk_vault.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
            decimals,
        )?;

        let clock = Clock::get()?;
        let token_asset_state = &mut ctx.accounts.token_asset_state;
        token_asset_state.balance_commitment = balance_commitment_after;
        token_asset_state.encrypted_balance = new_encrypted_balance;
        token_asset_state.updated_at = clock.unix_timestamp;

        let deposit_record = &mut ctx.accounts.token_deposit_record;
        require!(!deposit_record.is_withdrawn, SentinelError::AlreadyWithdrawn);
        deposit_record.is_withdrawn = true;
        deposit_record.withdrawn_at = clock.unix_timestamp;

        Ok(())
    }

    pub fn get_balance(_ctx: Context<GetBalance>) -> Result<()> {
        Ok(())
    }
}

#[account]
pub struct RailState {
    pub authority: Pubkey,
    pub institution_type: u8,
    pub compliance_level: u8,
    pub is_sealed: bool,
    pub is_active: bool,
    pub is_paused: bool,
    pub _padding: [u8; 2],
    pub audit_seal: [u8; 32],
    pub total_handshakes: u64,
    pub created_at: i64,
    pub sealed_at: i64,
    pub deactivated_at: i64,
    pub deactivation_reason: u8,
    pub version: u8,
    pub _reserved: [u8; 6],
}

#[account]
pub struct ZkVault {
    pub rail: Pubkey,
    pub elgamal_pubkey: [u8; 32],
    pub encrypted_balance: [u8; 64],
    pub balance_commitment: [u8; 32],
    pub deposit_count: u64,
    pub token_deposit_count: u64,
    pub bump: u8,
}

#[account]
pub struct VaultAssetState {
    pub rail: Pubkey,
    pub asset_key: [u8; 32],
    pub balance_commitment: [u8; 32],
    pub encrypted_balance: [u8; 64],
    pub updated_at: i64,
    pub bump: u8,
}

#[account]
pub struct HandshakeState {
    pub rail: Pubkey,
    pub commitment: [u8; 32],
    pub nullifier_hash: [u8; 32],
    pub is_active: bool,
    pub _padding: [u8; 7],
    pub created_at: i64,
    pub revoked_at: i64,
}

#[account]
pub struct NullifierRegistry {
    pub rail: Pubkey,
    pub nullifier_hash: [u8; 32],
    pub is_spent: bool,
    pub _padding: [u8; 7],
    pub spent_at: i64,
}

#[account]
pub struct DepositRecord {
    pub rail: Pubkey,
    pub sender: Pubkey,
    pub encrypted_amount: [u8; 64],
    pub commitment: [u8; 32],
    pub is_withdrawn: bool,
    pub _padding: [u8; 7],
    pub created_at: i64,
    pub withdrawn_at: i64,
    pub bump: u8,
}

#[account]
pub struct TokenDepositRecord {
    pub rail: Pubkey,
    pub sender: Pubkey,
    pub token_mint: Pubkey,
    pub encrypted_amount: [u8; 64],
    pub commitment: [u8; 32],
    pub decimals: u8,
    pub is_withdrawn: bool,
    pub _padding: [u8; 6],
    pub created_at: i64,
    pub withdrawn_at: i64,
    pub bump: u8,
}

#[account]
pub struct TransferRecord {
    pub sender_rail: Pubkey,
    pub receiver_rail: Pubkey,
    pub sender_commitment: [u8; 32],
    pub receiver_commitment: [u8; 32],
    pub nullifier_hash: [u8; 32],
    pub proof_hash: [u8; 32],
    pub is_token: bool,
    pub token_mint: Pubkey,
    pub created_at: i64,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct InitializeRail<'info> {
    #[account(
        init,
        payer = authority,
        space = 119,
        seeds = [b"rail", authority.key().as_ref()],
        bump
    )]
    pub rail: Account<'info, RailState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        constraint = authority_token_account.owner == authority.key() @ SentinelError::InvalidTokenAccount,
        constraint = authority_token_account.mint == north_mint.key() @ SentinelError::InvalidMint
    )]
    pub authority_token_account: InterfaceAccount<'info, TokenAccount>,
    pub north_mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeZkVault<'info> {
    #[account(
        init,
        payer = authority,
        space = 185,
        seeds = [b"zk_vault", rail.key().as_ref()],
        bump
    )]
    pub zk_vault: Account<'info, ZkVault>,
    #[account(has_one = authority @ SentinelError::Unauthorized)]
    pub rail: Account<'info, RailState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(commitment: [u8; 32], nullifier_hash: [u8; 32])]
pub struct CreateHandshake<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 32 + 1 + 7 + 8 + 8,
        seeds = [b"handshake", rail.key().as_ref(), nullifier_hash.as_ref()],
        bump
    )]
    pub handshake: Account<'info, HandshakeState>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 1 + 7 + 8,
        seeds = [b"nullifier", rail.key().as_ref(), nullifier_hash.as_ref()],
        bump
    )]
    pub nullifier_registry: Account<'info, NullifierRegistry>,
    #[account(mut)]
    pub rail: Account<'info, RailState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SealRail<'info> {
    #[account(mut, has_one = authority @ SentinelError::Unauthorized)]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(reason_code: u8)]
pub struct DeactivateRail<'info> {
    #[account(mut, has_one = authority @ SentinelError::Unauthorized)]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct PauseRail<'info> {
    #[account(mut, has_one = authority @ SentinelError::Unauthorized)]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UnpauseRail<'info> {
    #[account(mut, has_one = authority @ SentinelError::Unauthorized)]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(reason_code: u8)]
pub struct RevokeHandshake<'info> {
    #[account(mut)]
    pub handshake: Account<'info, HandshakeState>,
    #[account(has_one = authority @ SentinelError::Unauthorized)]
    pub rail: Account<'info, RailState>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        constraint = rail.is_active @ SentinelError::RailInactive,
        has_one = authority @ SentinelError::Unauthorized,
    )]
    pub rail: Account<'info, RailState>,
    #[account(
        mut,
        seeds = [b"zk_vault", rail.key().as_ref()],
        bump = zk_vault.bump,
    )]
    pub zk_vault: Account<'info, ZkVault>,
    #[account(
        init_if_needed,
        payer = sender,
        space = 177,
        seeds = [b"asset_vault", rail.key().as_ref(), SOL_ASSET_SEED],
        bump,
    )]
    pub sol_asset_state: Account<'info, VaultAssetState>,
    #[account(
        constraint = handshake.rail == rail.key() @ SentinelError::InvalidRail,
        constraint = handshake.is_active @ SentinelError::HandshakeAlreadyRevoked,
    )]
    pub handshake: Account<'info, HandshakeState>,
    #[account(
        mut,
        seeds = [b"vault_pool", rail.key().as_ref()],
        bump,
    )]
    /// CHECK: PDA vault pool, validated by seeds
    pub vault_pool: UncheckedAccount<'info>,
    #[account(
        init,
        payer = sender,
        space = 193,
        seeds = [b"deposit", rail.key().as_ref(), sender.key().as_ref(), &zk_vault.deposit_count.to_le_bytes()],
        bump,
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    #[account(mut)]
    pub sender: Signer<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(transfer_nonce: i64)]
pub struct ConfidentialTransfer<'info> {
    #[account(
        constraint = sender_rail.is_active @ SentinelError::RailInactive,
        has_one = authority @ SentinelError::Unauthorized
    )]
    pub sender_rail: Account<'info, RailState>,
    #[account(
        constraint = receiver_rail.is_active @ SentinelError::RailInactive,
        constraint = receiver_rail.authority == receiver_authority.key() @ SentinelError::Unauthorized
    )]
    pub receiver_rail: Account<'info, RailState>,
    #[account(
        mut,
        seeds = [b"zk_vault", sender_rail.key().as_ref()],
        bump = sender_zk_vault.bump,
    )]
    pub sender_zk_vault: Account<'info, ZkVault>,
    #[account(
        mut,
        seeds = [b"zk_vault", receiver_rail.key().as_ref()],
        bump = receiver_zk_vault.bump,
    )]
    pub receiver_zk_vault: Account<'info, ZkVault>,
    #[account(
        mut,
        seeds = [b"asset_vault", sender_rail.key().as_ref(), SOL_ASSET_SEED],
        bump = sender_sol_asset_state.bump,
        constraint = sender_sol_asset_state.rail == sender_rail.key() @ SentinelError::InvalidAssetState,
        constraint = sender_sol_asset_state.asset_key == sol_asset_key() @ SentinelError::InvalidAssetState,
    )]
    pub sender_sol_asset_state: Account<'info, VaultAssetState>,
    #[account(
        mut,
        seeds = [b"asset_vault", receiver_rail.key().as_ref(), SOL_ASSET_SEED],
        bump = receiver_sol_asset_state.bump,
        constraint = receiver_sol_asset_state.rail == receiver_rail.key() @ SentinelError::InvalidAssetState,
        constraint = receiver_sol_asset_state.asset_key == sol_asset_key() @ SentinelError::InvalidAssetState,
    )]
    pub receiver_sol_asset_state: Account<'info, VaultAssetState>,
    #[account(
        init,
        payer = authority,
        space = 242,
        seeds = [
            b"transfer",
            sender_rail.key().as_ref(),
            receiver_rail.key().as_ref(),
            &transfer_nonce.to_le_bytes()
        ],
        bump,
    )]
    pub transfer_record: Account<'info, TransferRecord>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub receiver_authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(constraint = rail.is_active @ SentinelError::RailInactive)]
    pub rail: Account<'info, RailState>,
    #[account(
        mut,
        seeds = [b"zk_vault", rail.key().as_ref()],
        bump = zk_vault.bump,
    )]
    pub zk_vault: Account<'info, ZkVault>,
    #[account(
        mut,
        seeds = [b"asset_vault", rail.key().as_ref(), SOL_ASSET_SEED],
        bump = sol_asset_state.bump,
        constraint = sol_asset_state.rail == rail.key() @ SentinelError::InvalidAssetState,
        constraint = sol_asset_state.asset_key == sol_asset_key() @ SentinelError::InvalidAssetState,
    )]
    pub sol_asset_state: Account<'info, VaultAssetState>,
    #[account(
        mut,
        constraint = deposit_record.rail == rail.key() @ SentinelError::InvalidRail,
        constraint = !deposit_record.is_withdrawn @ SentinelError::AlreadyWithdrawn,
    )]
    pub deposit_record: Account<'info, DepositRecord>,
    #[account(
        mut,
        seeds = [b"vault_pool", rail.key().as_ref()],
        bump,
        constraint = *vault_pool.owner == crate::ID @ SentinelError::InvalidVaultPoolOwner,
    )]
    /// CHECK: PDA vault pool, validated by seeds
    pub vault_pool: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: Receiver of withdrawn funds
    pub receiver: UncheckedAccount<'info>,
    #[account(constraint = rail.authority == authority.key() @ SentinelError::Unauthorized)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositToken<'info> {
    #[account(
        constraint = rail.is_active @ SentinelError::RailInactive,
        has_one = authority @ SentinelError::Unauthorized,
    )]
    pub rail: Box<Account<'info, RailState>>,
    #[account(
        mut,
        seeds = [b"zk_vault", rail.key().as_ref()],
        bump = zk_vault.bump,
    )]
    pub zk_vault: Box<Account<'info, ZkVault>>,
    #[account(
        constraint = handshake.rail == rail.key() @ SentinelError::InvalidRail,
        constraint = handshake.is_active @ SentinelError::HandshakeAlreadyRevoked,
    )]
    pub handshake: Box<Account<'info, HandshakeState>>,
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        init_if_needed,
        payer = sender,
        space = 177,
        seeds = [b"asset_vault", rail.key().as_ref(), token_mint.key().as_ref()],
        bump,
    )]
    pub token_asset_state: Box<Account<'info, VaultAssetState>>,
    #[account(
        mut,
        constraint = sender_token_account.owner == sender.key() @ SentinelError::InvalidTokenAccount,
        constraint = sender_token_account.mint == token_mint.key() @ SentinelError::InvalidMint,
    )]
    pub sender_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = vault_token_account.owner == zk_vault.key() @ SentinelError::InvalidTokenAccount,
        constraint = vault_token_account.mint == token_mint.key() @ SentinelError::InvalidMint,
    )]
    pub vault_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init,
        payer = sender,
        space = 225,
        seeds = [
            b"token_deposit",
            rail.key().as_ref(),
            sender.key().as_ref(),
            token_mint.key().as_ref(),
            &zk_vault.token_deposit_count.to_le_bytes()
        ],
        bump,
    )]
    pub token_deposit_record: Box<Account<'info, TokenDepositRecord>>,
    #[account(mut)]
    pub sender: Signer<'info>,
    pub authority: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(transfer_nonce: i64)]
pub struct ConfidentialTransferToken<'info> {
    #[account(
        constraint = sender_rail.is_active @ SentinelError::RailInactive,
        has_one = authority @ SentinelError::Unauthorized
    )]
    pub sender_rail: Box<Account<'info, RailState>>,
    #[account(
        constraint = receiver_rail.is_active @ SentinelError::RailInactive,
        constraint = receiver_rail.authority == receiver_authority.key() @ SentinelError::Unauthorized
    )]
    pub receiver_rail: Box<Account<'info, RailState>>,
    #[account(
        mut,
        seeds = [b"zk_vault", sender_rail.key().as_ref()],
        bump = sender_zk_vault.bump,
    )]
    pub sender_zk_vault: Box<Account<'info, ZkVault>>,
    #[account(
        mut,
        seeds = [b"zk_vault", receiver_rail.key().as_ref()],
        bump = receiver_zk_vault.bump,
    )]
    pub receiver_zk_vault: Box<Account<'info, ZkVault>>,
    pub token_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        seeds = [b"asset_vault", sender_rail.key().as_ref(), token_mint.key().as_ref()],
        bump = sender_token_asset_state.bump,
        constraint = sender_token_asset_state.rail == sender_rail.key() @ SentinelError::InvalidAssetState,
        constraint = sender_token_asset_state.asset_key == mint_asset_key(&token_mint.key()) @ SentinelError::InvalidAssetState,
    )]
    pub sender_token_asset_state: Box<Account<'info, VaultAssetState>>,
    #[account(
        mut,
        seeds = [b"asset_vault", receiver_rail.key().as_ref(), token_mint.key().as_ref()],
        bump = receiver_token_asset_state.bump,
        constraint = receiver_token_asset_state.rail == receiver_rail.key() @ SentinelError::InvalidAssetState,
        constraint = receiver_token_asset_state.asset_key == mint_asset_key(&token_mint.key()) @ SentinelError::InvalidAssetState,
    )]
    pub receiver_token_asset_state: Box<Account<'info, VaultAssetState>>,
    #[account(
        init,
        payer = authority,
        space = 242,
        seeds = [
            b"transfer",
            sender_rail.key().as_ref(),
            receiver_rail.key().as_ref(),
            &transfer_nonce.to_le_bytes()
        ],
        bump,
    )]
    pub transfer_record: Box<Account<'info, TransferRecord>>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub receiver_authority: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawToken<'info> {
    #[account(constraint = rail.is_active @ SentinelError::RailInactive)]
    pub rail: Account<'info, RailState>,
    #[account(
        mut,
        seeds = [b"zk_vault", rail.key().as_ref()],
        bump = zk_vault.bump,
    )]
    pub zk_vault: Account<'info, ZkVault>,
    #[account(
        mut,
        constraint = token_deposit_record.rail == rail.key() @ SentinelError::InvalidRail,
        constraint = !token_deposit_record.is_withdrawn @ SentinelError::AlreadyWithdrawn,
    )]
    pub token_deposit_record: Account<'info, TokenDepositRecord>,
    pub token_mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        seeds = [b"asset_vault", rail.key().as_ref(), token_mint.key().as_ref()],
        bump = token_asset_state.bump,
        constraint = token_asset_state.rail == rail.key() @ SentinelError::InvalidAssetState,
        constraint = token_asset_state.asset_key == mint_asset_key(&token_mint.key()) @ SentinelError::InvalidAssetState,
    )]
    pub token_asset_state: Account<'info, VaultAssetState>,
    #[account(
        mut,
        constraint = vault_token_account.owner == zk_vault.key() @ SentinelError::InvalidTokenAccount,
        constraint = vault_token_account.mint == token_mint.key() @ SentinelError::InvalidMint,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        constraint = receiver_token_account.mint == token_mint.key() @ SentinelError::InvalidMint,
    )]
    pub receiver_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(constraint = rail.authority == authority.key() @ SentinelError::Unauthorized)]
    pub authority: Signer<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetBalance<'info> {
    #[account(
        seeds = [b"zk_vault", rail.key().as_ref()],
        bump = zk_vault.bump,
    )]
    pub zk_vault: Account<'info, ZkVault>,
    pub rail: Account<'info, RailState>,
}

#[error_code]
pub enum SentinelError {
    #[msg("This privacy rail has been deactivated")]
    RailInactive,
    #[msg("Unauthorized: You are not the authority")]
    Unauthorized,
    #[msg("This nullifier has already been used")]
    NullifierAlreadyUsed,
    #[msg("This rail has been sealed")]
    RailSealed,
    #[msg("This rail is already sealed")]
    RailAlreadySealed,
    #[msg("This rail has already been deactivated")]
    RailAlreadyDeactivated,
    #[msg("This rail is paused")]
    RailPaused,
    #[msg("This rail is already paused")]
    RailAlreadyPaused,
    #[msg("This rail is not paused")]
    RailNotPaused,
    #[msg("This handshake has already been revoked")]
    HandshakeAlreadyRevoked,
    #[msg("Invalid rail for this handshake")]
    InvalidRail,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Authority must hold NORTH tokens")]
    InsufficientNorthTokens,
    #[msg("Invalid token account")]
    InvalidTokenAccount,
    #[msg("Invalid mint")]
    InvalidMint,
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Insufficient balance in vault")]
    InsufficientVaultBalance,
    #[msg("This deposit has already been withdrawn")]
    AlreadyWithdrawn,
    #[msg("Invalid ZK proof — verification failed")]
    InvalidZkProof,
    #[msg("Wrong number of public inputs for this proof")]
    InvalidProofInputs,
    #[msg("Groth16 pairing check failed")]
    ProofVerificationFailed,
    #[msg("On-chain commitment does not match provided commitment")]
    CommitmentMismatch,
    #[msg("Invalid vault pool owner")]
    InvalidVaultPoolOwner,
    #[msg("Invalid vault asset state")]
    InvalidAssetState,
    #[msg("Invalid transfer nonce")]
    InvalidTransferNonce,
}