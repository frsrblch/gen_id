use crate::Component;
use iter_context::ContextualIterator;
use std::ops::AddAssign;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

impl<C, T> Vector2<Component<C, T>> {
    pub fn iter(&self) -> impl ContextualIterator<Context = C> + IntoIterator<Item = Vector2<&T>> {
        self.x.iter().zip(&self.y).map(|(x, y)| Vector2 { x, y })
    }

    pub fn iter_mut(
        &mut self,
    ) -> impl ContextualIterator<Context = C> + IntoIterator<Item = Vector2<&mut T>> {
        self.x
            .iter_mut()
            .zip(&mut self.y)
            .map(|(x, y)| Vector2 { x, y })
    }
}

impl<T, U> AddAssign<Vector2<U>> for Vector2<T>
where
    T: AddAssign<U>,
{
    fn add_assign(&mut self, rhs: Vector2<U>) {
        self.x.add_assign(rhs.x);
        self.y.add_assign(rhs.y);
    }
}

impl<'a, T, U> AddAssign<&'a Vector2<U>> for Vector2<T>
where
    T: AddAssign<&'a U>,
{
    fn add_assign(&mut self, rhs: &'a Vector2<U>) {
        self.x.add_assign(&rhs.x);
        self.y.add_assign(&rhs.y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::Stat;

    #[test]
    fn vector2_comp_add_assign_ref() {
        let mut lhs = Vector2 {
            x: Component::<Stat, f32>::from(vec![0., 0., 0.]),
            y: Component::<Stat, f32>::from(vec![0., 0., 0.]),
        };
        let rhs = Vector2 {
            x: Component::<Stat, f32>::from(vec![1., 2., 3.]),
            y: Component::<Stat, f32>::from(vec![4., 5., 6.]),
        };

        lhs.add_assign(&rhs);

        assert_eq!(lhs, rhs);
    }

    #[test]
    fn vector2_comp_add_assign() {
        let mut lhs = Vector2 {
            x: Component::<Stat, f32>::from(vec![0., 0., 0.]),
            y: Component::<Stat, f32>::from(vec![0., 0., 0.]),
        };
        let rhs = Vector2 {
            x: Component::<Stat, f32>::from(vec![1., 2., 3.]),
            y: Component::<Stat, f32>::from(vec![4., 5., 6.]),
        };

        lhs.add_assign(rhs.clone());

        assert_eq!(lhs, rhs);
    }
}
