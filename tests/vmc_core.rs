use alfatch_vmc::vmc::vmc_core::{ExtractedId, extract_game_id_from_save};

#[test]
fn test_extract_game_id_from_save() {
    let test_cases = vec![
        (
            "BESLES55673SAVEDATA",
            ExtractedId {
                id: "BESLES55673".to_string(),
                suffix: "SAVEDATA".to_string(),
            },
        ),
        (
            "BASLUS21050DAT0",
            ExtractedId {
                id: "BASLUS21050".to_string(),
                suffix: "DAT0".to_string(),
            },
        ),
        (
            "BASCUS97436",
            ExtractedId {
                id: "BASCUS97436".to_string(),
                suffix: "".to_string(),
            },
        ),
        (
            "UNKNOWN_FORMAT",
            ExtractedId {
                id: "UNKNOWN_FORMAT".to_string(),
                suffix: "".to_string(),
            },
        ),
    ];

    for (input, expected) in test_cases {
        let result = extract_game_id_from_save(input);
        assert_eq!(result.id, expected.id);
        assert_eq!(result.suffix, expected.suffix);
    }
}
//
// #[test]
// fn test_get_game_title_with_fallback() {
//     let test_cases = vec![
//         (
//             "BESLES55673SAVEDATA",
//             "PES 2014: Pro Evolution Soccer (SAVEDATA)",
//         ),
//         ("BASLUS21050DAT0", "Burnout 3: Takedown (DAT0)"),
//         ("BASCUS97436", "Gran Turismo 4"),
//         ("UNKNOWN_ID", "Unknown Game (UNKNOWN_ID)"),
//     ];
//
//     for (input, expected) in test_cases {
//         let result = get_game_title(input);
//         assert_eq!(result, expected);
//     }
// }
//
// #[test]
// fn test_fs_entry_from_raw() {
//     use alfatch_vmc::model::vmc_core_model::RawFSEntry;
//
//     // Create a minimal valid raw entry
//     let mut raw_entry = RawFSEntry {
//         mode: 0x8497, // Typical file mode
//         _pad1: 0,
//         length: 1024,
//         created_sec: 30,
//         created_min: 15,
//         created_hour: 10,
//         created_day: 5,
//         created_month: 12,
//         _pad2: 0,
//         created_year: 2023,
//         cluster: 100,
//         dir_entry: 0,
//         modified_sec: 45,
//         modified_min: 30,
//         modified_hour: 14,
//         modified_day: 6,
//         modified_month: 12,
//         _pad3: 0,
//         modified_year: 2023,
//         attr: 0,
//         _pad4: [0; 28],
//         name: [0; 32],
//         _pad5: [0; 412],
//     };
//
//     // Set a valid name
//     let name_bytes = "TEST_SAVE".as_bytes();
//     raw_entry.name[..name_bytes.len()].copy_from_slice(name_bytes);
//
//     // Test conversion
//     let fs_entry = FSEntry::from_raw(&raw_entry).unwrap();
//
//     assert_eq!(fs_entry.name, "TEST_SAVE");
//     assert_eq!(fs_entry.length, 1024);
//     assert_eq!(fs_entry.cluster, 100);
//     assert!(!fs_entry.is_directory);
// }
//
// #[test]
// fn test_vmc_superblock_parsing() {
//     // Create a valid superblock in memory
//     let mut data = Vec::new();
//
//     // Magic string (28 bytes)
//     data.extend_from_slice(b"Sony PS2 Memory Card Format ");
//     data.resize(28, 0);
//
//     // Version (12 bytes)
//     data.extend_from_slice(b"1.2.0.0");
//     data.resize(28 + 12, 0);
//
//     // Fill remaining required fields with appropriate values
//     data.resize(0x28, 0);
//     data.write_i16::<LittleEndian>(512).unwrap(); // page_size
//     data.write_u16::<LittleEndian>(2).unwrap(); // pages_per_cluster
//
//     data.resize(0x34, 0);
//     data.write_u32::<LittleEndian>(100).unwrap(); // alloc_offset
//
//     data.resize(0x3C, 0);
//     data.write_u32::<LittleEndian>(200).unwrap(); // rootdir_cluster
//     data.write_u32::<LittleEndian>(0).unwrap(); // backup_block1
//     data.write_u32::<LittleEndian>(0).unwrap(); // backup_block2
//
//     // Continue filling other required fields...
//     data.resize(0x154, 0);
//     data.write_u32::<LittleEndian>(1024).unwrap(); // cluster_size
//
//     data.resize(0x170, 0);
//     data.write_u32::<LittleEndian>(2048).unwrap(); // max_allocatable_clusters
//
//     let mut cursor = Cursor::new(data);
//     let superblock = VmcSuperblock::from_reader(&mut cursor);
//
//     assert!(superblock.is_ok());
//     let sb = superblock.unwrap();
//     assert_eq!(sb.magic, "Sony PS2 Memory Card Format ");
//     assert_eq!(sb.cluster_size, 1024);
// }
//
// // #[test]
// // fn test_fat_table_operations() {
// //     // Create a simple FAT table
// //     let fat = FatTable {
// //         fat: vec![
// //             0x7FFFFFFF, // Free cluster
// //             0x80000002, // Cluster 1 points to cluster 2
// //             0x80000003, // Cluster 2 points to cluster 3
// //             0xFFFFFFFF, // End of chain
// //         ],
// //     };
// //
// //     // Test cluster chain building
// //     let vmc = Vmc {
// //         file: todo!(), // We'd need a mock file for complete test
// //         superblock: todo!(),
// //         fat,
// //     };
// //
// //     let chain = vmc.build_cluster_chain(1);
// //     assert_eq!(chain, vec![1, 2, 3]);
// // }
// //
// // #[test]
// // fn test_free_cluster_counting() {
// //     let fat = FatTable {
// //         fat: vec![
// //             0x7FFFFFFF, // Free
// //             0x80000002, // Used
// //             0x80000003, // Used
// //             0x7FFFFFFF, // Free
// //             0xFFFFFFFF, // Used (end of chain)
// //         ],
// //     };
// //
// //     let vmc = Vmc {
// //         file: todo!(),
// //         superblock: todo!(),
// //         fat,
// //     };
// //
// //     assert_eq!(vmc.count_free_clusters(), 2);
// // }
