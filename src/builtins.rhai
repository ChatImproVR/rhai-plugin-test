fn quat(yaw, pitch, roll) {
    let sr = roll.sin();
    let cr = roll.cos();

    let sp = pitch.sin();
    let cp = pitch.cos();

    let sy = yaw.sin();
    let cy = yaw.cos();

    return [
        cr * cp * cy + sr * sp * sy,
        sr * cp * cy - cr * sp * sy,
        cr * sp * cy + sr * cp * sy,
        cr * cp * sy - sr * sp * cy,
    ];
}

