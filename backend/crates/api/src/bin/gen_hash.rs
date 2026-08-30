use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2
};

fn main() {
    let argon2 = Argon2::default();
    
    let password = "password123";
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap().to_string();
    println!("password123: {}", password_hash);

    let admin_password = "admin123";
    let admin_salt = SaltString::generate(&mut OsRng);
    let admin_hash = argon2.hash_password(admin_password.as_bytes(), &admin_salt).unwrap().to_string();
    println!("admin123: {}", admin_hash);
}
