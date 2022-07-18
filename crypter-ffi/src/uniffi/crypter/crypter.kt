// This file was autogenerated by some hot garbage in the `uniffi` crate.
// Trust me, you don't want to mess with it!

@file:Suppress("NAME_SHADOWING")

package uniffi.crypter;

// Common helper code.
//
// Ideally this would live in a separate .kt file where it can be unittested etc
// in isolation, and perhaps even published as a re-useable package.
//
// However, it's important that the detils of how this helper code works (e.g. the
// way that different builtin types are passed across the FFI) exactly match what's
// expected by the Rust code on the other side of the interface. In practice right
// now that means coming from the exact some version of `uniffi` that was used to
// compile the Rust component. The easiest way to ensure this is to bundle the Kotlin
// helpers directly inline like we're doing here.

import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure
import com.sun.jna.ptr.ByReference
import java.nio.ByteBuffer
import java.nio.ByteOrder

// This is a helper for safely working with byte buffers returned from the Rust code.
// A rust-owned buffer is represented by its capacity, its current length, and a
// pointer to the underlying data.

@Structure.FieldOrder("capacity", "len", "data")
open class RustBuffer : Structure() {
    @JvmField var capacity: Int = 0
    @JvmField var len: Int = 0
    @JvmField var data: Pointer? = null

    class ByValue : RustBuffer(), Structure.ByValue
    class ByReference : RustBuffer(), Structure.ByReference

    companion object {
        internal fun alloc(size: Int = 0) = rustCall() { status ->
            _UniFFILib.INSTANCE.ffi_crypter_9c38_rustbuffer_alloc(size, status).also {
                if(it.data == null) {
                   throw RuntimeException("RustBuffer.alloc() returned null data pointer (size=${size})")
               }
            }
        }

        internal fun free(buf: RustBuffer.ByValue) = rustCall() { status ->
            _UniFFILib.INSTANCE.ffi_crypter_9c38_rustbuffer_free(buf, status)
        }
    }

    @Suppress("TooGenericExceptionThrown")
    fun asByteBuffer() =
        this.data?.getByteBuffer(0, this.len.toLong())?.also {
            it.order(ByteOrder.BIG_ENDIAN)
        }
}

/**
 * The equivalent of the `*mut RustBuffer` type.
 * Required for callbacks taking in an out pointer.
 *
 * Size is the sum of all values in the struct.
 */
class RustBufferByReference : ByReference(16) {
    /**
     * Set the pointed-to `RustBuffer` to the given value.
     */
    fun setValue(value: RustBuffer.ByValue) {
        // NOTE: The offsets are as they are in the C-like struct.
        val pointer = getPointer()
        pointer.setInt(0, value.capacity)
        pointer.setInt(4, value.len)
        pointer.setPointer(8, value.data)
    }
}

// This is a helper for safely passing byte references into the rust code.
// It's not actually used at the moment, because there aren't many things that you
// can take a direct pointer to in the JVM, and if we're going to copy something
// then we might as well copy it into a `RustBuffer`. But it's here for API
// completeness.

@Structure.FieldOrder("len", "data")
open class ForeignBytes : Structure() {
    @JvmField var len: Int = 0
    @JvmField var data: Pointer? = null

    class ByValue : ForeignBytes(), Structure.ByValue
}
// The FfiConverter interface handles converter types to and from the FFI
//
// All implementing objects should be public to support external types.  When a
// type is external we need to import it's FfiConverter.
public interface FfiConverter<KotlinType, FfiType> {
    // Convert an FFI type to a Kotlin type
    fun lift(value: FfiType): KotlinType

    // Convert an Kotlin type to an FFI type
    fun lower(value: KotlinType): FfiType

    // Read a Kotlin type from a `ByteBuffer`
    fun read(buf: ByteBuffer): KotlinType

    // Calculate bytes to allocate when creating a `RustBuffer`
    //
    // This must return at least as many bytes as the write() function will
    // write. It can return more bytes than needed, for example when writing
    // Strings we can't know the exact bytes needed until we the UTF-8
    // encoding, so we pessimistically allocate the largest size possible (3
    // bytes per codepoint).  Allocating extra bytes is not really a big deal
    // because the `RustBuffer` is short-lived.
    fun allocationSize(value: KotlinType): Int

    // Write a Kotlin type to a `ByteBuffer`
    fun write(value: KotlinType, buf: ByteBuffer)

    // Lower a value into a `RustBuffer`
    //
    // This method lowers a value into a `RustBuffer` rather than the normal
    // FfiType.  It's used by the callback interface code.  Callback interface
    // returns are always serialized into a `RustBuffer` regardless of their
    // normal FFI type.
    fun lowerIntoRustBuffer(value: KotlinType): RustBuffer.ByValue {
        val rbuf = RustBuffer.alloc(allocationSize(value))
        try {
            val bbuf = rbuf.data!!.getByteBuffer(0, rbuf.capacity.toLong()).also {
                it.order(ByteOrder.BIG_ENDIAN)
            }
            write(value, bbuf)
            rbuf.writeField("len", bbuf.position())
            return rbuf
        } catch (e: Throwable) {
            RustBuffer.free(rbuf)
            throw e
        }
    }

    // Lift a value from a `RustBuffer`.
    //
    // This here mostly because of the symmetry with `lowerIntoRustBuffer()`.
    // It's currently only used by the `FfiConverterRustBuffer` class below.
    fun liftFromRustBuffer(rbuf: RustBuffer.ByValue): KotlinType {
        val byteBuf = rbuf.asByteBuffer()!!
        try {
           val item = read(byteBuf)
           if (byteBuf.hasRemaining()) {
               throw RuntimeException("junk remaining in buffer after lifting, something is very wrong!!")
           }
           return item
        } finally {
            RustBuffer.free(rbuf)
        }
    }
}

// FfiConverter that uses `RustBuffer` as the FfiType
public interface FfiConverterRustBuffer<KotlinType>: FfiConverter<KotlinType, RustBuffer.ByValue> {
    override fun lift(value: RustBuffer.ByValue) = liftFromRustBuffer(value)
    override fun lower(value: KotlinType) = lowerIntoRustBuffer(value)
}
// A handful of classes and functions to support the generated data structures.
// This would be a good candidate for isolating in its own ffi-support lib.
// Error runtime.
@Structure.FieldOrder("code", "error_buf")
internal open class RustCallStatus : Structure() {
    @JvmField var code: Int = 0
    @JvmField var error_buf: RustBuffer.ByValue = RustBuffer.ByValue()

    fun isSuccess(): Boolean {
        return code == 0
    }

    fun isError(): Boolean {
        return code == 1
    }

    fun isPanic(): Boolean {
        return code == 2
    }
}

class InternalException(message: String) : Exception(message)

// Each top-level error class has a companion object that can lift the error from the call status's rust buffer
interface CallStatusErrorHandler<E> {
    fun lift(error_buf: RustBuffer.ByValue): E;
}

// Helpers for calling Rust
// In practice we usually need to be synchronized to call this safely, so it doesn't
// synchronize itself

// Call a rust function that returns a Result<>.  Pass in the Error class companion that corresponds to the Err
private inline fun <U, E: Exception> rustCallWithError(errorHandler: CallStatusErrorHandler<E>, callback: (RustCallStatus) -> U): U {
    var status = RustCallStatus();
    val return_value = callback(status)
    if (status.isSuccess()) {
        return return_value
    } else if (status.isError()) {
        throw errorHandler.lift(status.error_buf)
    } else if (status.isPanic()) {
        // when the rust code sees a panic, it tries to construct a rustbuffer
        // with the message.  but if that code panics, then it just sends back
        // an empty buffer.
        if (status.error_buf.len > 0) {
            throw InternalException(FfiConverterString.lift(status.error_buf))
        } else {
            throw InternalException("Rust panic")
        }
    } else {
        throw InternalException("Unknown rust call status: $status.code")
    }
}

// CallStatusErrorHandler implementation for times when we don't expect a CALL_ERROR
object NullCallStatusErrorHandler: CallStatusErrorHandler<InternalException> {
    override fun lift(error_buf: RustBuffer.ByValue): InternalException {
        RustBuffer.free(error_buf)
        return InternalException("Unexpected CALL_ERROR")
    }
}

// Call a rust function that returns a plain value
private inline fun <U> rustCall(callback: (RustCallStatus) -> U): U {
    return rustCallWithError(NullCallStatusErrorHandler, callback);
}

// Contains loading, initialization code,
// and the FFI Function declarations in a com.sun.jna.Library.
@Synchronized
private fun findLibraryName(componentName: String): String {
    val libOverride = System.getProperty("uniffi.component.$componentName.libraryOverride")
    if (libOverride != null) {
        return libOverride
    }
    return "crypter"
}

private inline fun <reified Lib : Library> loadIndirect(
    componentName: String
): Lib {
    return Native.load<Lib>(findLibraryName(componentName), Lib::class.java)
}

// A JNA Library to expose the extern-C FFI definitions.
// This is an implementation detail which will be called internally by the public API.

internal interface _UniFFILib : Library {
    companion object {
        internal val INSTANCE: _UniFFILib by lazy {
            loadIndirect<_UniFFILib>(componentName = "crypter")
            
        }
    }

    fun crypter_9c38_pubkey_from_secret_key(`mySecretKey`: RustBuffer.ByValue,
    _uniffi_out_err: RustCallStatus
    ): RustBuffer.ByValue

    fun crypter_9c38_derive_shared_secret(`theirPubkey`: RustBuffer.ByValue,`mySecretKey`: RustBuffer.ByValue,
    _uniffi_out_err: RustCallStatus
    ): RustBuffer.ByValue

    fun crypter_9c38_encrypt(`plaintext`: RustBuffer.ByValue,`secret`: RustBuffer.ByValue,`nonce`: RustBuffer.ByValue,
    _uniffi_out_err: RustCallStatus
    ): RustBuffer.ByValue

    fun crypter_9c38_decrypt(`ciphertext`: RustBuffer.ByValue,`secret`: RustBuffer.ByValue,
    _uniffi_out_err: RustCallStatus
    ): RustBuffer.ByValue

    fun ffi_crypter_9c38_rustbuffer_alloc(`size`: Int,
    _uniffi_out_err: RustCallStatus
    ): RustBuffer.ByValue

    fun ffi_crypter_9c38_rustbuffer_from_bytes(`bytes`: ForeignBytes.ByValue,
    _uniffi_out_err: RustCallStatus
    ): RustBuffer.ByValue

    fun ffi_crypter_9c38_rustbuffer_free(`buf`: RustBuffer.ByValue,
    _uniffi_out_err: RustCallStatus
    ): Unit

    fun ffi_crypter_9c38_rustbuffer_reserve(`buf`: RustBuffer.ByValue,`additional`: Int,
    _uniffi_out_err: RustCallStatus
    ): RustBuffer.ByValue

    
}

// Public interface members begin here.


public object FfiConverterString: FfiConverter<String, RustBuffer.ByValue> {
    // Note: we don't inherit from FfiConverterRustBuffer, because we use a
    // special encoding when lowering/lifting.  We can use `RustBuffer.len` to
    // store our length and avoid writing it out to the buffer.
    override fun lift(value: RustBuffer.ByValue): String {
        try {
            val byteArr = ByteArray(value.len)
            value.asByteBuffer()!!.get(byteArr)
            return byteArr.toString(Charsets.UTF_8)
        } finally {
            RustBuffer.free(value)
        }
    }

    override fun read(buf: ByteBuffer): String {
        val len = buf.getInt()
        val byteArr = ByteArray(len)
        buf.get(byteArr)
        return byteArr.toString(Charsets.UTF_8)
    }

    override fun lower(value: String): RustBuffer.ByValue {
        val byteArr = value.toByteArray(Charsets.UTF_8)
        // Ideally we'd pass these bytes to `ffi_bytebuffer_from_bytes`, but doing so would require us
        // to copy them into a JNA `Memory`. So we might as well directly copy them into a `RustBuffer`.
        val rbuf = RustBuffer.alloc(byteArr.size)
        rbuf.asByteBuffer()!!.put(byteArr)
        return rbuf
    }

    // We aren't sure exactly how many bytes our string will be once it's UTF-8
    // encoded.  Allocate 3 bytes per unicode codepoint which will always be
    // enough.
    override fun allocationSize(value: String): Int {
        val sizeForLength = 4
        val sizeForString = value.length * 3
        return sizeForLength + sizeForString
    }

    override fun write(value: String, buf: ByteBuffer) {
        val byteArr = value.toByteArray(Charsets.UTF_8)
        buf.putInt(byteArr.size)
        buf.put(byteArr)
    }
}





sealed class CrypterException(message: String): Exception(message) {
        // Each variant is a nested class
        // Flat enums carries a string error message, so no special implementation is necessary.
        class DerivePublicKey(message: String) : CrypterException(message)
        class DeriveSharedSecret(message: String) : CrypterException(message)
        class Encrypt(message: String) : CrypterException(message)
        class Decrypt(message: String) : CrypterException(message)
        class BadPubkey(message: String) : CrypterException(message)
        class BadSecret(message: String) : CrypterException(message)
        class BadNonce(message: String) : CrypterException(message)
        class BadCiper(message: String) : CrypterException(message)
        

    companion object ErrorHandler : CallStatusErrorHandler<CrypterException> {
        override fun lift(error_buf: RustBuffer.ByValue): CrypterException = FfiConverterTypeCrypterError.lift(error_buf)
    }
}

public object FfiConverterTypeCrypterError : FfiConverterRustBuffer<CrypterException> {
    override fun read(buf: ByteBuffer): CrypterException {
        
            return when(buf.getInt()) {
            1 -> CrypterException.DerivePublicKey(FfiConverterString.read(buf))
            2 -> CrypterException.DeriveSharedSecret(FfiConverterString.read(buf))
            3 -> CrypterException.Encrypt(FfiConverterString.read(buf))
            4 -> CrypterException.Decrypt(FfiConverterString.read(buf))
            5 -> CrypterException.BadPubkey(FfiConverterString.read(buf))
            6 -> CrypterException.BadSecret(FfiConverterString.read(buf))
            7 -> CrypterException.BadNonce(FfiConverterString.read(buf))
            8 -> CrypterException.BadCiper(FfiConverterString.read(buf))
            else -> throw RuntimeException("invalid error enum value, something is very wrong!!")
        }
        
    }

    @Suppress("UNUSED_PARAMETER")
    override fun allocationSize(value: CrypterException): Int {
        throw RuntimeException("Writing Errors is not supported")
    }

    @Suppress("UNUSED_PARAMETER")
    override fun write(value: CrypterException, buf: ByteBuffer) {
        throw RuntimeException("Writing Errors is not supported")
    }

}
@Throws(CrypterException::class)

fun `pubkeyFromSecretKey`(`mySecretKey`: String): String {
    return FfiConverterString.lift(
    rustCallWithError(CrypterException) { _status ->
    _UniFFILib.INSTANCE.crypter_9c38_pubkey_from_secret_key(FfiConverterString.lower(`mySecretKey`), _status)
})
}


@Throws(CrypterException::class)

fun `deriveSharedSecret`(`theirPubkey`: String, `mySecretKey`: String): String {
    return FfiConverterString.lift(
    rustCallWithError(CrypterException) { _status ->
    _UniFFILib.INSTANCE.crypter_9c38_derive_shared_secret(FfiConverterString.lower(`theirPubkey`), FfiConverterString.lower(`mySecretKey`), _status)
})
}


@Throws(CrypterException::class)

fun `encrypt`(`plaintext`: String, `secret`: String, `nonce`: String): String {
    return FfiConverterString.lift(
    rustCallWithError(CrypterException) { _status ->
    _UniFFILib.INSTANCE.crypter_9c38_encrypt(FfiConverterString.lower(`plaintext`), FfiConverterString.lower(`secret`), FfiConverterString.lower(`nonce`), _status)
})
}


@Throws(CrypterException::class)

fun `decrypt`(`ciphertext`: String, `secret`: String): String {
    return FfiConverterString.lift(
    rustCallWithError(CrypterException) { _status ->
    _UniFFILib.INSTANCE.crypter_9c38_decrypt(FfiConverterString.lower(`ciphertext`), FfiConverterString.lower(`secret`), _status)
})
}



