use super::Job;

pub enum Filter {
    All {
        exclude: Vec<String>,
    },
    Subset {
        jobs: Vec<String>,
        groups: Vec<String>,
        exclude: Vec<String>,
    },
}

impl Filter {
    pub fn matches(&self, job: &Job) -> bool {
        match self {
            Self::All { exclude } => !exclude.contains(&job.name),
            Self::Subset {
                groups,
                jobs,
                exclude,
            } => {
                if exclude.contains(&job.name) {
                    return false;
                }

                groups.contains(&job.group) || jobs.contains(&job.name)
            }
        }
    }
}
