//====================================================================

use shipyard::Component;

//====================================================================

#[derive(Component)]
#[track(All)]
pub struct Transform {
    pub translation: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        }
    }
}

impl Transform {
    #[inline]
    pub fn from_translation(translation: glam::Vec3) -> Self {
        Self {
            translation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_rotation(rotation: glam::Quat) -> Self {
        Self {
            rotation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_scale(scale: glam::Vec3) -> Self {
        Self {
            scale,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_translation_rotatation(translation: glam::Vec3, rotation: glam::Quat) -> Self {
        Self {
            translation,
            rotation,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_translation_scale(translation: glam::Vec3, scale: glam::Vec3) -> Self {
        Self {
            translation,
            scale,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_rotation_scale(rotation: glam::Quat, scale: glam::Vec3) -> Self {
        Self {
            rotation,
            scale,
            ..Default::default()
        }
    }

    #[inline]
    pub fn from_translation_rotatation_scale(
        translation: glam::Vec3,
        rotation: glam::Quat,
        scale: glam::Vec3,
    ) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }
}

impl Transform {
    #[inline]
    pub fn forward(&self) -> glam::Vec3 {
        self.rotation * glam::Vec3::Z
    }

    #[inline]
    pub fn right(&self) -> glam::Vec3 {
        self.rotation * glam::Vec3::X
    }

    pub fn lerp(&mut self, target: &Transform, s: f32) {
        self.translation = self.translation.lerp(target.translation, s);
        self.rotation = self.rotation.lerp(target.rotation, s);
        self.scale = self.scale.lerp(target.scale, s);
    }

    #[inline]
    pub fn to_array(&self) -> [f32; 16] {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
            .to_cols_array()
    }

    #[inline]
    pub fn to_normal_matrix_array(&self) -> [f32; 9] {
        glam::Mat3::from_quat(self.rotation).to_cols_array()
    }
}

//--------------------------------------------------

// TODO - Review these operations
impl std::ops::Add for Transform {
    type Output = Self;

    fn add(mut self, rhs: Transform) -> Self::Output {
        self.translation += rhs.translation;
        self.rotation = self.rotation.mul_quat(rhs.rotation);
        self.scale *= rhs.scale;
        self
    }
}

impl std::ops::AddAssign for Transform {
    fn add_assign(&mut self, rhs: Self) {
        self.translation += rhs.translation;
        self.rotation = self.rotation.mul_quat(rhs.rotation);
        self.scale *= rhs.scale;
    }
}

impl std::ops::Sub for Transform {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self.translation -= rhs.translation;
        self.rotation = self.rotation.mul_quat(rhs.rotation.inverse());
        self.scale /= rhs.scale;

        self
    }
}

//====================================================================
