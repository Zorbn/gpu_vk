pub type Mat4 = [f32; 16];

pub struct OrthographicProjectionInfo {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub z_near: f32,
    pub z_far: f32,
}

pub fn orthographic_projection(mat4: &mut Mat4, info: OrthographicProjectionInfo) {
    mat4[0] = 2.0 / (info.right - info.left);
    mat4[1] = 0.0;
    mat4[2] = 0.0;
    mat4[3] = 0.0;

    mat4[4] = 0.0;
    mat4[5] = 2.0 / (info.top - info.bottom);
    mat4[6] = 0.0;
    mat4[7] = 0.0;

    mat4[8] = 0.0;
    mat4[9] = 0.0;
    mat4[10] = 1.0 / (info.z_near - info.z_far);
    mat4[11] = 0.0;

    mat4[12] = (info.right + info.left) / (info.left - info.right);
    mat4[13] = (info.top + info.bottom) / (info.bottom - info.top);
    mat4[14] = info.z_near / (info.z_near - info.z_far);
    mat4[15] = 1.0;
}
