fn main() -> Result<(), Box<dyn std::error::Error>> {
    punctum_bundler::run(&punctum_bundler::Config::for_current_package())
}
