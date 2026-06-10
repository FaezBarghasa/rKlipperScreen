use rklipperscreen::system::touch::CalibrationMatrix;

#[test]
fn test_calibration_identity() {
    let matrix = CalibrationMatrix {
        a: 1.0, b: 0.0, c: 0.0,
        d: 0.0, e: 1.0, f: 0.0,
    };
    
    assert_eq!(matrix.calibrate_point(100.0, 200.0), (100, 200));
    assert_eq!(matrix.calibrate_point(0.0, 0.0), (0, 0));
}

#[test]
fn test_calibration_transform() {
    // Example: Screen mapping from a 0-4096 touch sensor to an 800x480 pixel display resolution.
    // A = 800 / 4096 = 0.1953125
    // E = 480 / 4096 = 0.1171875
    let matrix = CalibrationMatrix {
        a: 0.1953125, b: 0.0, c: 0.0,
        d: 0.0, e: 0.1171875, f: 0.0,
    };
    
    assert_eq!(matrix.calibrate_point(2048.0, 2048.0), (400, 240));
    assert_eq!(matrix.calibrate_point(0.0, 0.0), (0, 0));
    assert_eq!(matrix.calibrate_point(4096.0, 4096.0), (800, 480));
}

#[test]
fn test_calibration_negative_clamping() {
    let matrix = CalibrationMatrix {
        a: 1.0, b: 0.0, c: -50.0,
        d: 0.0, e: 1.0, f: -50.0,
    };
    
    // Should safely bound to 0 to prevent u32 underflow panics on screen coordinates.
    assert_eq!(matrix.calibrate_point(10.0, 10.0), (0, 0));
}