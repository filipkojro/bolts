package app;

public class Main {
    static {
        System.loadLibrary("bolts");
    }

    public static native int add(int a, int b);
    public static native void convertToBolt(String filename);

    public static void main(String[] args) {
        int result = add(15, 27);
        System.out.println("Result from Rust: " + result);

        convertToBolt("large-file.dat");
    }
}
