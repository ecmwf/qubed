use roaring::bitmap::RoaringBitmap;

#[test]
pub fn benchmark_roaring_vs_sorted_array() {
    // let mut data1 = vec![
    //     0, 8, 9, 10, 31, 33, 34, 39, 40, 41, 42, 49, 60, 75, 76, 78, 79, 129, 130, 131, 132,
    //     133, 134, 135, 136, 137, 139, 141, 142, 144, 146, 147, 148, 151, 157, 159, 164, 165,
    //     166, 167, 168, 169, 170, 172, 175, 176, 177, 178, 179, 180, 181, 182, 183, 186, 187,
    //     188, 195, 196, 201, 202, 205, 207, 208, 209, 210, 211, 212, 228, 235, 236, 246, 247,
    //     248, 3008, 3020, 3062, 3073, 3074, 3075, 140221, 140229, 140230, 140231, 140232,
    //     174096, 228002, 228021, 228023, 228024, 228029, 228038, 228050, 228051, 228058, 228089,
    //     228090, 228129, 228141, 228143, 228144, 228164, 228216, 228218, 228219, 228221, 228228,
    //     228235, 228236, 228246, 228247, 228251, 231001, 231002, 231040, 231041, 231044, 231045,
    //     231046, 231047, 231048, 231049, 231057, 231058, 231067, 231070, 235015, 235017, 235018,
    //     235019, 235020, 235021, 235031, 235033, 235034, 235035, 235036, 235037, 235038, 235039,
    //     235040, 235041, 235042, 235043, 235049, 235050, 235051, 235052, 235053, 235055, 235071,
    //     237120, 238105, 238382, 260015, 260028, 260046, 260048, 260058, 260109, 260155, 260199,
    //     260238, 260242, 260259, 260318, 260320, 260360, 260646, 260647, 260654, 260655, 261002,
    //     263000, 263001, 263002, 263003, 263004, 263008, 263009, 263021, 263022, 263100, 263101,
    //     263114, 263115, 263121, 263122, 263123, 263124, 263500, 263501, 263505, 263506, 263507,
    // ];

    let mut data1 = vec![
        0, 8, 9, 10, 31, 33, 34, 39, 40, 41, 42, 49, 60, 75, 76, 78, 79, 129, 130, 131, 132,
        133, 134, 135, 136, 137, 139, 141, 142, 144, 146, 147, 148, 151, 157, 159, 164, 165,
        166, 167, 168, 169, 170, 172, 175, 176, 177, 178, 179, 180, 181, 182, 183, 186, 187,
        188, 195, 196, 201, 202, 205, 207, 208, 209, 210, 211, 212, 228, 235, 236, 246, 247,
        248
    ];

    // Replicate 5x with offsets
    let original_len = data1.len() as u32;
    
    for i in 1..100 {
        let offset = i * 300;
        for i in 0..original_len {
            data1.push(data1[i as usize] + offset);
        }
    }

    data1.sort();
    data1.dedup();

    let mut data2: Vec<u32> = data1
        .iter()
        .step_by(2)
        .copied()
        .chain(vec![1, 2, 3, 4, 5, 6, 7].into_iter())
        .collect();
    data2.sort();
    data2.dedup();

    // println!("Data 1: {:?}", data1);
    // println!("Data 2: {:?}", data2);

    // === Setup Roaring Bitmaps ===
    let mut bitmap1 = RoaringBitmap::new();
    for &val in &data1 {
        bitmap1.insert(val);
    }

    let mut bitmap2 = RoaringBitmap::new();
    for &val in &data2 {
        bitmap2.insert(val);
    }

    println!("=== Data Setup ===");
    println!("Data1 size: {} elements", data1.len());
    println!("Data2 size: {} elements", data2.len());
    println!("Bitmap1 cardinality: {}", bitmap1.len());
    println!("Bitmap2 cardinality: {}", bitmap2.len());
    println!();

    // Helper function for sorted array intersection using two pointers
    fn array_intersection(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < a.len() && j < b.len() {
            if a[i] == b[j] {
                result.push(a[i]);
                i += 1;
                j += 1;
            } else if a[i] < b[j] {
                i += 1;
            } else {
                j += 1;
            }
        }
        result
    }

    // Helper function for sorted array union
    fn array_union(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < a.len() && j < b.len() {
            if a[i] == b[j] {
                result.push(a[i]);
                i += 1;
                j += 1;
            } else if a[i] < b[j] {
                result.push(a[i]);
                i += 1;
            } else {
                result.push(b[j]);
                j += 1;
            }
        }
        result.extend_from_slice(&a[i..]);
        result.extend_from_slice(&b[j..]);
        result
    }

    // Helper function for sorted array difference
    fn array_difference(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < a.len() {
            if j >= b.len() {
                result.extend_from_slice(&a[i..]);
                break;
            }

            if a[i] == b[j] {
                i += 1;
                j += 1;
            } else if a[i] < b[j] {
                result.push(a[i]);
                i += 1;
            } else {
                j += 1;
            }
        }
        result
    }

    // Helper function for sorted array symmetric difference
    fn array_sym_diff(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < a.len() && j < b.len() {
            if a[i] == b[j] {
                i += 1;
                j += 1;
            } else if a[i] < b[j] {
                result.push(a[i]);
                i += 1;
            } else {
                result.push(b[j]);
                j += 1;
            }
        }
        result.extend_from_slice(&a[i..]);
        result.extend_from_slice(&b[j..]);
        result
    }

    println!("=== MEMORY ===");
    println!(
        "Roaring bitmap1 serialized size: ~{} bytes",
        bitmap1.serialized_size()
    );
    println!(
        "Sorted array1 size: {} bytes",
        data1.len() * std::mem::size_of::<u32>()
);

    println!("=== INTERSECTION ===");
    // Roaring intersection
    let start = std::time::Instant::now();
    let mut roaring_result = None;
    for _ in 0..10000 {
        roaring_result = Some(&bitmap1 & &bitmap2);
    }
    let roaring_time = start.elapsed();
    let roaring_intersection = roaring_result.unwrap();
    println!(
        "Roaring (10k iterations): {:?}, result cardinality: {}",
        roaring_time,
        roaring_intersection.len()
    );

    // Array intersection
    let start = std::time::Instant::now();
    let mut array_result = Vec::new();
    for _ in 0..10000 {
        array_result = array_intersection(&data1, &data2);
    }
    let array_time = start.elapsed();
    println!(
        "Sorted Array (10k iterations): {:?}, result cardinality: {}",
        array_time,
        array_result.len()
    );
    println!("Roaring is {:.2}x faster", array_time.as_secs_f64() / roaring_time.as_secs_f64());
    println!();

    println!("=== UNION ===");
    // Roaring union
    let start = std::time::Instant::now();
    let mut roaring_result = None;
    for _ in 0..10000 {
        roaring_result = Some(&bitmap1 | &bitmap2);
    }
    let roaring_time = start.elapsed();
    let roaring_union = roaring_result.unwrap();
    println!(
        "Roaring (10k iterations): {:?}, result cardinality: {}",
        roaring_time,
        roaring_union.len()
    );

    // Array union
    let start = std::time::Instant::now();
    let mut array_result = Vec::new();
    for _ in 0..10000 {
        array_result = array_union(&data1, &data2);
    }
    let array_time = start.elapsed();
    println!(
        "Sorted Array (10k iterations): {:?}, result cardinality: {}",
        array_time,
        array_result.len()
    );
    println!("Roaring is {:.2}x faster", array_time.as_secs_f64() / roaring_time.as_secs_f64());
    println!();

    println!("=== DIFFERENCE ===");
    // Roaring difference
    let start = std::time::Instant::now();
    let mut roaring_result = None;
    for _ in 0..10000 {
        roaring_result = Some(&bitmap1 - &bitmap2);
    }
    let roaring_time = start.elapsed();
    let roaring_difference = roaring_result.unwrap();
    println!(
        "Roaring (10k iterations): {:?}, result cardinality: {}",
        roaring_time,
        roaring_difference.len()
    );

    // Array difference
    let start = std::time::Instant::now();
    let mut array_result = Vec::new();
    for _ in 0..10000 {
        array_result = array_difference(&data1, &data2);
    }
    let array_time = start.elapsed();
    println!(
        "Sorted Array (10k iterations): {:?}, result cardinality: {}",
        array_time,
        array_result.len()
    );
    println!("Roaring is {:.2}x faster", array_time.as_secs_f64() / roaring_time.as_secs_f64());
    println!();

    println!("=== SYMMETRIC DIFFERENCE ===");
    // Roaring symmetric difference
    let start = std::time::Instant::now();
    let mut roaring_result = None;
    for _ in 0..10000 {
        roaring_result = Some(&bitmap1 ^ &bitmap2);
    }
    let roaring_time = start.elapsed();
    let roaring_sym_diff = roaring_result.unwrap();
    println!(
        "Roaring (10k iterations): {:?}, result cardinality: {}",
        roaring_time,
        roaring_sym_diff.len()
    );

    // Array symmetric difference
    let start = std::time::Instant::now();
    let mut array_result = Vec::new();
    for _ in 0..10000 {
        array_result = array_sym_diff(&data1, &data2);
    }
    let array_time = start.elapsed();
    println!(
        "Sorted Array (10k iterations): {:?}, result cardinality: {}",
        array_time,
        array_result.len()
    );
    println!("Roaring is {:.2}x faster", array_time.as_secs_f64() / roaring_time.as_secs_f64());
    println!();

    println!("=== MEMORY ===");
    println!(
        "Roaring bitmap1 serialized size: ~{} bytes",
        bitmap1.serialized_size()
    );
    println!(
        "Sorted array1 size: {} bytes",
        data1.len() * std::mem::size_of::<u32>()
    );
}