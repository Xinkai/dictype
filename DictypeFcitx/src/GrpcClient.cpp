#include "GrpcClient.h"
#include "DictypeLog.h"

GrpcClient::GrpcClient(Dictype::Dictype::Stub* stub, ResponseCb onResponse,
                       DoneCb onDone, const std::string& profileName)
    : onResponse_(std::move(onResponse)), onDone_(std::move(onDone)) {
    Dictype::TranscribeRequest request;
    request.set_profile_name(profileName);
    stub->async()->Transcribe(&context_, &request, this);
    StartRead(&response_);
    StartCall();
}

void GrpcClient::OnDone(const grpc::Status& s) {
    if (s.error_code()) {
        DICTYPE_ERROR() << "GrpcClient::OnDone ErrorCode:" << s.error_code()
                        << " ErrorMessage:" << s.error_message();
    }

    try {
        onDone_(s);
    } catch (...) {
        DICTYPE_ERROR() << "Exception in onDone_ callback.";
    }

    delete this;
}

void GrpcClient::OnReadDone(const bool ok) {
    if (!ok) {
        return;
    }

    try {
        onResponse_(response_);
    } catch (...) {
        DICTYPE_ERROR() << "Exception in onResponse_ callback.";
    }

    StartRead(&response_);
}
