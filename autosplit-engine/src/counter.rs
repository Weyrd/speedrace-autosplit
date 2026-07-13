// A single tick adds at most MAX_COUNTER_STEP -> prevents bugged value
pub const MAX_COUNTER_STEP: i32 = 1000;

pub struct Counter {
    run: i64,          // total count for the run
    last: Option<i32>, //  delta calculation
    emitted: i64,      // last emitted value
}

impl Counter {
    pub fn new() -> Self {
        Counter {
            run: 0,
            last: None,
            emitted: -1,
        }
    }

    pub fn reset(&mut self) {
        *self = Counter::new();
    }

    pub fn feed(&mut self, v: Option<i32>) {
        if let Some(v) = v {
            if let Some(prev) = self.last {
                let step = v - prev;
                if step > 0 && step <= MAX_COUNTER_STEP {
                    self.run += step as i64;
                }
            }
            self.last = Some(v);
        }
    }

    pub fn take_emit(&mut self) -> Option<i64> {
        (self.run != self.emitted).then(|| {
            self.emitted = self.run;
            self.run
        })
    }
}

impl Default for Counter {
    fn default() -> Self {
        Counter::new()
    }
}

pub struct EdgeCounter {
    run: i64,
    prev: bool,
    emitted: i64,
}

impl EdgeCounter {
    pub fn new() -> Self {
        EdgeCounter {
            run: 0,
            prev: false,
            emitted: -1,
        }
    }

    pub fn reset(&mut self) {
        *self = EdgeCounter::new();
    }

    pub fn feed(&mut self, flag: bool) {
        if flag && !self.prev {
            self.run += 1;
        }
        self.prev = flag;
    }

    pub fn take_emit(&mut self) -> Option<i64> {
        (self.run != self.emitted).then(|| {
            self.emitted = self.run;
            self.run
        })
    }
}

impl Default for EdgeCounter {
    fn default() -> Self {
        EdgeCounter::new()
    }
}
