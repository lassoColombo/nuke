use ./apps.nu

export def main [] {
  {
    apps: {
      v1: {
        deployments: {
          |
          owner: record, 
          parents: list, 
          revision: int, 
          output?: string
          |
          apps deployments v1 $owner $parents $revision $output
        }
        controllerrevisions: {
          |
          owner: record, 
          parents: list, 
          revision: int, 
          output?: string
          |
          apps controllerrevisions v1 $owner $parents $revision $output
        }
      }
    }
  }
}

