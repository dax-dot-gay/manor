macro_rules! action {
    ($name:ident) => {
        pub fn $name(self) -> Self {
            Self(self.0.$name())
        }
    };

    ($param:ident:$type:ty) => {
        $param: $type
    };


    ($param:ident:$type:ty, $($params:ident:$types:ty),+) => {
        $param:$type, action!($($params:$types),+)
    };

    ($name:ident, $($param:ident:$type:ty),+) => {
        pub fn $name(self, action!($($param:$type),+)) -> Self {
            Self(self.0.$name())
        }
    }
}

action!(session, key:impl Into<&'a mut ClientSession>, beans: String);
