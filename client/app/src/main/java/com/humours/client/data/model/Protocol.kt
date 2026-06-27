package com.humours.client.data.model

import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonDecoder
import kotlinx.serialization.json.JsonEncoder
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.double
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import kotlinx.serialization.json.longOrNull

@Serializable
enum class MetricDataType {
    @SerialName("float") Float,
    @SerialName("integer") Integer,
    @SerialName("boolean") BooleanMetric,
    @SerialName("string") StringMetric,
    @SerialName("stringlist") StringList,
}

@Serializable
data class CatalogMetric(
    val id: String,
    val name: String,
    @SerialName("default_unit") val defaultUnit: String,
    @SerialName("available_units") val availableUnits: List<String>,
    @SerialName("static") val isStatic: Boolean = false,
    @SerialName("data_type") val dataType: MetricDataType,
)

@Serializable
data class CatalogMessage(
    @SerialName("type") val msgType: String,
    val metrics: List<CatalogMetric>,
)

@Serializable
data class SubscribeEntry(
    val id: String,
    @SerialName("refresh_rate_ms") val refreshRateMs: Long? = null,
    val unit: String? = null,
)

@Serializable
data class SubscribeMessage(
    @SerialName("type") val msgType: String,
    val metrics: List<SubscribeEntry>,
)

@Serializable(with = MetricNumberSerializer::class)
sealed class MetricNumber {
    fun asF64(): Float = when (this) {
        is FloatValue -> value
        is IntegerValue -> value.toFloat()
        is BooleanValue -> if (value) 1f else 0f
        is StringValue -> 0f
        is StringListValue -> 0f
    }

    fun asString(): String = when (this) {
        is FloatValue -> "%.2f".format(value)
        is IntegerValue -> value.toString()
        is BooleanValue -> value.toString()
        is StringValue -> value
        is StringListValue -> value.joinToString(",")
    }
}

data class FloatValue(val value: Float) : MetricNumber()
data class IntegerValue(val value: Long) : MetricNumber()
data class BooleanValue(val value: Boolean) : MetricNumber()
data class StringValue(val value: String) : MetricNumber()
data class StringListValue(val value: List<String>) : MetricNumber()

object MetricNumberSerializer : KSerializer<MetricNumber> {
    override val descriptor: SerialDescriptor =
        PrimitiveSerialDescriptor("MetricNumber", PrimitiveKind.STRING)

    override fun deserialize(decoder: Decoder): MetricNumber {
        val jsonDecoder = decoder as? JsonDecoder
            ?: throw IllegalStateException("MetricNumber requires JSON input")
        return parseElement(jsonDecoder.decodeJsonElement())
    }

    private fun parseElement(element: kotlinx.serialization.json.JsonElement): MetricNumber =
        when (element) {
            is JsonPrimitive -> {
                element.booleanOrNull?.let { return BooleanValue(it) }
                element.longOrNull?.let { return IntegerValue(it) }
                element.doubleOrNull?.let { return FloatValue(it.toFloat()) }
                element.contentOrNull?.let { return StringValue(it) }
                throw IllegalStateException("Unparseable MetricNumber primitive: $element")
            }
            is JsonArray -> StringListValue(element.map { it.jsonPrimitive.content })
            is JsonObject -> throw IllegalStateException("Unexpected object as MetricNumber: $element")
            else -> throw IllegalStateException("Unexpected MetricNumber element: $element")
        }

    override fun serialize(encoder: Encoder, value: MetricNumber) {
        val jsonEncoder = encoder as? JsonEncoder
            ?: throw IllegalStateException("MetricNumber requires JSON output")
        val element: kotlinx.serialization.json.JsonElement = when (value) {
            is FloatValue -> JsonPrimitive(value.value)
            is IntegerValue -> JsonPrimitive(value.value)
            is BooleanValue -> JsonPrimitive(value.value)
            is StringValue -> JsonPrimitive(value.value)
            is StringListValue -> buildJsonArray { value.value.forEach { add(JsonPrimitive(it)) } }
        }
        jsonEncoder.encodeJsonElement(element)
    }
}

@Serializable
data class MetricValue(
    val id: String,
    val value: MetricNumber,
    val unit: String,
)

@Serializable
data class DataMessage(
    @SerialName("type") val msgType: String,
    val timestamp: Long,
    val metrics: List<MetricValue>,
)

@Serializable
data class ErrorMessage(
    @SerialName("type") val msgType: String,
    val message: String,
)