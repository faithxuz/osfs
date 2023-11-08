pub trait Bitmap {
    fn get_map(&self, pos: u32) -> u64;
    fn set_map(&mut self, pos: u32, map: u64);
    fn next_usable(&self) -> Option<u32>;

    fn check(&self, pos: u32) -> bool {
        let map = self.get_map(pos);
        let flag: u64 = 1 << (pos % 64);
        map & flag > 0
    }

    fn set_true(&mut self, pos: u32) {
        let map = self.get_map(pos);
        let flag: u64 = 1 << (pos % 64);
        self.set_map(pos, map | flag);
    }

    fn set_false(&mut self, pos: u32) {
        let map = self.get_map(pos);
        let flag: u64 = !(1 << (pos % 64));
        self.set_map(pos, map | !flag);
    }
}