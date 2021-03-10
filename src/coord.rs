#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Coord(pub i32, pub i32);

impl Coord {
    pub fn dot(self, rhs: Self) -> i32 {
        self.0 * rhs.0 + self.1 * rhs.1
    }
    pub fn length_squared(self) -> i32 {
        self.0 * self.0 + self.1 * self.1
    }
    pub fn mul_components(self, rhs: Self) -> Self {
        Self(self.0 * rhs.0, self.1 * rhs.1)
    }
}

impl std::ops::Add for Coord {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0, self.1 + rhs.1)
    }
}

impl std::ops::AddAssign for Coord {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl std::ops::Sub for Coord {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0, self.1 - rhs.1)
    }
}

impl std::ops::SubAssign for Coord {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

impl std::ops::Neg for Coord {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0, -self.1)
    }
}

impl std::ops::Mul<i32> for Coord {
    type Output = Self;
    fn mul(self, rhs: i32) -> Self {
        Self(self.0 * rhs, self.1 * rhs)
    }
}

impl std::ops::MulAssign<i32> for Coord {
    fn mul_assign(&mut self, rhs: i32) {
        self.0 *= rhs;
        self.1 *= rhs;
    }
}
