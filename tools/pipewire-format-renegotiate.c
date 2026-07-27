// SPDX-FileCopyrightText: 2026 AE-5 Control contributors
// SPDX-License-Identifier: MIT

#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include <pipewire/pipewire.h>
#include <spa/param/audio/format-utils.h>
#include <spa/utils/result.h>

#define DEFAULT_DWELL_MS 300U
#define DEFAULT_CYCLES 1U
#define DEFAULT_NODE_NAME "ae5-format-renegotiation-probe"
#define CHANNELS 2U

struct format_spec {
	enum spa_audio_format format;
	uint32_t rate;
	const char *name;
};

static const struct format_spec format_sequence[] = {
	{ SPA_AUDIO_FORMAT_S16_LE, 44100, "S16LE" },
	{ SPA_AUDIO_FORMAT_S32_LE, 48000, "S32LE" },
	{ SPA_AUDIO_FORMAT_S32_LE, 96000, "S32LE" },
	{ SPA_AUDIO_FORMAT_S16_LE, 48000, "S16LE" },
};

struct options {
	const char *target;
	const char *node_name;
	unsigned int dwell_ms;
	unsigned int cycles;
	bool unlinked;
};

struct data {
	struct pw_main_loop *loop;
	struct pw_stream *stream;
	struct spa_source *timer;
	struct options options;
	unsigned int sequence_index;
	unsigned int updates;
	unsigned int negotiated;
	unsigned int bytes_per_sample;
	bool timer_armed;
	bool awaiting_negotiation;
	bool started;
	bool streaming;
	bool stopping;
	int result;
};

static void usage(FILE *stream, const char *program)
{
	fprintf(stream,
		"usage: %s --target NODE_NAME|0 [--dwell-ms 100..5000] "
		"[--cycles 1..100] [--node-name NAME]\n"
		"\n"
		"Produces digital silence and changes the advertised format in one "
		"PipeWire client.\n"
		"Target 0 creates an unlinked graph probe; every other target enables "
		"autoconnect.\n",
		program);
}

static int parse_unsigned(const char *text, unsigned int minimum,
			  unsigned int maximum, unsigned int *value)
{
	char *end = NULL;
	unsigned long parsed;

	errno = 0;
	parsed = strtoul(text, &end, 10);
	if (errno != 0 || end == text || *end != '\0' || parsed < minimum ||
	    parsed > maximum)
		return -EINVAL;
	*value = (unsigned int)parsed;
	return 0;
}

static int parse_options(int argc, char *argv[], struct options *options)
{
	int index;

	*options = (struct options){
		.node_name = DEFAULT_NODE_NAME,
		.dwell_ms = DEFAULT_DWELL_MS,
		.cycles = DEFAULT_CYCLES,
	};
	for (index = 1; index < argc; index++) {
		if (strcmp(argv[index], "--help") == 0) {
			usage(stdout, argv[0]);
			return 1;
		}
		if (index + 1 >= argc) {
			fprintf(stderr, "error: missing value for %s\n", argv[index]);
			return -EINVAL;
		}
		if (strcmp(argv[index], "--target") == 0) {
			options->target = argv[++index];
		} else if (strcmp(argv[index], "--dwell-ms") == 0) {
			if (parse_unsigned(argv[++index], 100, 5000,
					   &options->dwell_ms) < 0) {
				fprintf(stderr, "error: invalid dwell time\n");
				return -EINVAL;
			}
		} else if (strcmp(argv[index], "--cycles") == 0) {
			if (parse_unsigned(argv[++index], 1, 100,
					   &options->cycles) < 0) {
				fprintf(stderr, "error: invalid cycle count\n");
				return -EINVAL;
			}
		} else if (strcmp(argv[index], "--node-name") == 0) {
			options->node_name = argv[++index];
		} else {
			fprintf(stderr, "error: unknown option: %s\n", argv[index]);
			return -EINVAL;
		}
	}
	if (options->target == NULL || options->target[0] == '\0') {
		fprintf(stderr, "error: --target is required\n");
		return -EINVAL;
	}
	if (options->node_name[0] == '\0') {
		fprintf(stderr, "error: --node-name cannot be empty\n");
		return -EINVAL;
	}
	options->unlinked = strcmp(options->target, "0") == 0;
	return 0;
}

static void log_format(const char *event, const struct format_spec *spec)
{
	fprintf(stdout, "%s format=%s rate=%u channels=%u\n", event,
		spec->name, spec->rate, CHANNELS);
	fflush(stdout);
}

static const struct spa_pod *
build_format(struct spa_pod_builder *builder, const struct format_spec *spec)
{
	struct spa_audio_info_raw info = SPA_AUDIO_INFO_RAW_INIT(
		.format = spec->format,
		.rate = spec->rate,
		.channels = CHANNELS,
		.position = { SPA_AUDIO_CHANNEL_FL, SPA_AUDIO_CHANNEL_FR });

	return spa_format_audio_raw_build(builder, SPA_PARAM_EnumFormat, &info);
}

static void disarm_timer(struct data *data)
{
	if (data->timer == NULL || !data->timer_armed)
		return;
	pw_loop_update_timer(pw_main_loop_get_loop(data->loop), data->timer,
			     NULL, NULL, false);
	data->timer_armed = false;
}

static void arm_timer(struct data *data)
{
	struct timespec timeout = {
		.tv_sec = data->options.dwell_ms / 1000,
		.tv_nsec = (long)(data->options.dwell_ms % 1000) * 1000000L,
	};

	if (data->timer_armed)
		return;
	if (pw_loop_update_timer(pw_main_loop_get_loop(data->loop), data->timer,
				 &timeout, NULL, false) < 0) {
		fprintf(stderr, "error: unable to arm renegotiation timer\n");
		data->result = EXIT_FAILURE;
		pw_main_loop_quit(data->loop);
		return;
	}
	data->timer_armed = true;
}

static void on_process(void *userdata)
{
	struct data *data = userdata;
	struct pw_buffer *pw_buffer;
	struct spa_buffer *buffer;
	struct spa_data *audio;
	uint32_t frames;
	uint32_t stride;

	pw_buffer = pw_stream_dequeue_buffer(data->stream);
	if (pw_buffer == NULL)
		return;
	buffer = pw_buffer->buffer;
	if (buffer->n_datas == 0)
		goto queue;
	audio = &buffer->datas[0];
	if (audio->data == NULL || audio->chunk == NULL)
		goto queue;

	stride = data->bytes_per_sample * CHANNELS;
	frames = audio->maxsize / stride;
	if (pw_buffer->requested > 0 && pw_buffer->requested < frames)
		frames = (uint32_t)pw_buffer->requested;
	memset(audio->data, 0, (size_t)frames * stride);
	audio->chunk->offset = 0;
	audio->chunk->stride = (int32_t)stride;
	audio->chunk->size = frames * stride;

queue:
	pw_stream_queue_buffer(data->stream, pw_buffer);
}

static void on_param_changed(void *userdata, uint32_t id,
			     const struct spa_pod *param)
{
	struct data *data = userdata;
	struct spa_audio_info_raw info = { 0 };
	const struct format_spec *expected;

	if (id != SPA_PARAM_Format || param == NULL)
		return;
	if (spa_format_audio_raw_parse(param, &info) < 0) {
		fprintf(stderr, "error: PipeWire selected an invalid raw format\n");
		data->result = EXIT_FAILURE;
		pw_main_loop_quit(data->loop);
		return;
	}
	expected = &format_sequence[data->sequence_index];
	if (info.format != expected->format || info.rate != expected->rate ||
	    info.channels != CHANNELS) {
		fprintf(stderr,
			"error: unexpected format id=%u rate=%u channels=%u; "
			"expected %s/%u/%u\n",
			info.format, info.rate, info.channels, expected->name,
			expected->rate, CHANNELS);
		data->result = EXIT_FAILURE;
		pw_main_loop_quit(data->loop);
		return;
	}
	data->bytes_per_sample =
		info.format == SPA_AUDIO_FORMAT_S16_LE ? 2 : 4;
	if (data->awaiting_negotiation) {
		data->negotiated++;
		data->awaiting_negotiation = false;
	}
	fprintf(stdout, "negotiated format=%s rate=%u channels=%u\n",
		expected->name, info.rate, info.channels);
	fflush(stdout);
	if (!data->options.unlinked && data->started && data->streaming)
		arm_timer(data);
}

static void on_state_changed(void *userdata, enum pw_stream_state old,
			     enum pw_stream_state state, const char *error)
{
	struct data *data = userdata;

	(void)old;
	fprintf(stdout, "state=%s node_id=%u\n",
		pw_stream_state_as_string(state),
		pw_stream_get_node_id(data->stream));
	fflush(stdout);
	data->streaming = state == PW_STREAM_STATE_STREAMING;
	if (state == PW_STREAM_STATE_ERROR) {
		fprintf(stderr, "error: PipeWire stream failed: %s\n",
			error != NULL ? error : "unknown error");
		data->result = EXIT_FAILURE;
		pw_main_loop_quit(data->loop);
	} else if (state == PW_STREAM_STATE_UNCONNECTED && data->started &&
		   !data->stopping) {
		data->result = EXIT_FAILURE;
		pw_main_loop_quit(data->loop);
	} else if (state == PW_STREAM_STATE_STREAMING ||
		   (state == PW_STREAM_STATE_PAUSED && data->options.unlinked)) {
		data->started = true;
		arm_timer(data);
	} else if (state == PW_STREAM_STATE_PAUSED) {
		disarm_timer(data);
	}
}

static void on_timeout(void *userdata, uint64_t expirations)
{
	struct data *data = userdata;
	const struct spa_pod *params[1];
	struct spa_pod_builder builder;
	uint8_t buffer[512];
	unsigned int maximum_updates;
	int result;

	(void)expirations;
	data->timer_armed = false;
	maximum_updates =
		data->options.cycles * SPA_N_ELEMENTS(format_sequence);
	if (data->updates >= maximum_updates) {
		if (!data->options.unlinked &&
		    (data->awaiting_negotiation ||
		     data->negotiated != data->updates + 1)) {
			fprintf(stderr,
				"error: incomplete format negotiation: "
				"updates=%u negotiated=%u\n",
				data->updates, data->negotiated);
			data->result = EXIT_FAILURE;
			data->stopping = true;
			pw_main_loop_quit(data->loop);
			return;
		}
		fprintf(stdout, "complete updates=%u negotiated=%u\n",
			data->updates, data->negotiated);
		fflush(stdout);
		data->stopping = true;
		pw_main_loop_quit(data->loop);
		return;
	}

	data->sequence_index =
		(data->sequence_index + 1) % SPA_N_ELEMENTS(format_sequence);
	data->awaiting_negotiation = true;
	builder = SPA_POD_BUILDER_INIT(buffer, sizeof(buffer));
	params[0] = build_format(
		&builder, &format_sequence[data->sequence_index]);
	result = pw_stream_update_params(data->stream, params, 1);
	if (result < 0) {
		fprintf(stderr, "error: format update failed: %s\n",
			spa_strerror(result));
		data->result = EXIT_FAILURE;
		pw_main_loop_quit(data->loop);
		return;
	}
	data->updates++;
	log_format("announced", &format_sequence[data->sequence_index]);
	if (data->options.unlinked)
		arm_timer(data);
}

static void on_signal(void *userdata, int signal_number)
{
	struct data *data = userdata;

	(void)signal_number;
	data->result = 128;
	data->stopping = true;
	pw_main_loop_quit(data->loop);
}

static const struct pw_stream_events stream_events = {
	PW_VERSION_STREAM_EVENTS,
	.state_changed = on_state_changed,
	.param_changed = on_param_changed,
	.process = on_process,
};

int main(int argc, char *argv[])
{
	struct data data = {
		.awaiting_negotiation = true,
		.bytes_per_sample = 2,
		.result = EXIT_SUCCESS,
	};
	struct pw_properties *properties;
	const struct spa_pod *params[1];
	struct spa_pod_builder builder;
	uint8_t buffer[512];
	enum pw_stream_flags flags;
	int parse_result;
	int result;

	parse_result = parse_options(argc, argv, &data.options);
	if (parse_result > 0)
		return EXIT_SUCCESS;
	if (parse_result < 0) {
		usage(stderr, argv[0]);
		return EXIT_FAILURE;
	}

	pw_init(&argc, &argv);
	data.loop = pw_main_loop_new(NULL);
	if (data.loop == NULL) {
		fprintf(stderr, "error: unable to create PipeWire main loop\n");
		pw_deinit();
		return EXIT_FAILURE;
	}
	pw_loop_add_signal(pw_main_loop_get_loop(data.loop), SIGINT, on_signal,
			   &data);
	pw_loop_add_signal(pw_main_loop_get_loop(data.loop), SIGTERM, on_signal,
			   &data);
	data.timer = pw_loop_add_timer(pw_main_loop_get_loop(data.loop),
				       on_timeout, &data);
	if (data.timer == NULL) {
		fprintf(stderr, "error: unable to create renegotiation timer\n");
		pw_main_loop_destroy(data.loop);
		pw_deinit();
		return EXIT_FAILURE;
	}

	properties = pw_properties_new(
		PW_KEY_MEDIA_TYPE, "Audio",
		PW_KEY_MEDIA_CATEGORY, "Playback",
		PW_KEY_MEDIA_ROLE, "Test",
		PW_KEY_NODE_NAME, data.options.node_name,
		PW_KEY_NODE_DESCRIPTION, "AE-5 in-place format probe",
		NULL);
	if (properties == NULL) {
		fprintf(stderr, "error: unable to create PipeWire properties\n");
		pw_main_loop_destroy(data.loop);
		pw_deinit();
		return EXIT_FAILURE;
	}
	flags = PW_STREAM_FLAG_MAP_BUFFERS;
	if (!data.options.unlinked) {
		result = pw_properties_set(properties, PW_KEY_TARGET_OBJECT,
					   data.options.target);
		if (result < 0) {
			fprintf(stderr,
				"error: unable to set exact PipeWire target: %s\n",
				spa_strerror(result));
			pw_properties_free(properties);
			pw_main_loop_destroy(data.loop);
			pw_deinit();
			return EXIT_FAILURE;
		}
		flags |= PW_STREAM_FLAG_AUTOCONNECT;
	}
	data.stream = pw_stream_new_simple(
		pw_main_loop_get_loop(data.loop), "ae5-format-renegotiation",
		properties, &stream_events, &data);
	if (data.stream == NULL) {
		fprintf(stderr, "error: unable to create PipeWire stream\n");
		pw_main_loop_destroy(data.loop);
		pw_deinit();
		return EXIT_FAILURE;
	}

	builder = SPA_POD_BUILDER_INIT(buffer, sizeof(buffer));
	params[0] = build_format(&builder, &format_sequence[0]);
	log_format("initial", &format_sequence[0]);
	result = pw_stream_connect(data.stream, PW_DIRECTION_OUTPUT, PW_ID_ANY,
				   flags, params, 1);
	if (result < 0) {
		fprintf(stderr, "error: unable to connect stream: %s\n",
			spa_strerror(result));
		data.result = EXIT_FAILURE;
	} else {
		pw_main_loop_run(data.loop);
	}

	data.stopping = true;
	pw_stream_destroy(data.stream);
	pw_main_loop_destroy(data.loop);
	pw_deinit();
	return data.result;
}
