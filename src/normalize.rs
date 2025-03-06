pub trait Normalize {
    fn normalize(self) -> Self;
}

impl Normalize for Vec<f32> {
    fn normalize(self) -> Self {
        let total = self.iter().sum::<f32>();
        if total < f32::EPSILON {
            return self;
        }
        self.into_iter().map(|x| x / total).collect()
    }
}
